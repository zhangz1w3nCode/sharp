//! parser.rs - frontmatter 与 wiki-link 解析。
//!
//! wiki-link 语法:
//!   [[`.knowledges/xxx.md`]]              无标签边
//!   [[`.knowledges/xxx.md`|关系]]         带标签边
//!   [[.knowledges/xxx.md|关系]]           兼容不带反引号
//!   [[.knowledges/xxx.md]]                兼容不带反引号

use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

/// wiki-link 正则:覆盖四种形态(移植自 Python WIKILINK_RE)。
static WIKILINK_RE: OnceLock<Regex> = OnceLock::new();

fn wikilink_re() -> &'static Regex {
    WIKILINK_RE.get_or_init(|| {
        Regex::new(r"\[\[\s*`?([^`|\]]+?)`?(?:\|([^\]]+?))?\s*\]\]")
            .expect("WIKILINK_RE: 静态正则,编译期可验证,不会失败")
    })
}

/// 行内数组元素正则(移植自 Python _INLINE_ELEM_RE)。
static INLINE_ELEM_RE: OnceLock<Regex> = OnceLock::new();

fn inline_elem_re() -> &'static Regex {
    INLINE_ELEM_RE.get_or_init(|| {
        Regex::new(
            r#"\s*(?:"((?:\\.|[^"\\])*)"'|'((?:[^']|'')*)'|([^,]+?))\s*(?:,|$)"#,
        )
        .expect("inline elem regex")
    })
}

/// frontmatter 字段(只关心 5 个字段,其余忽略)。
#[derive(Debug, Clone, Serialize)]
pub struct Frontmatter {
    pub name: String,
    pub summary: String,
    pub domain: String,
    pub tags: Vec<String>,
    pub status: String,
}

impl Default for Frontmatter {
    fn default() -> Self {
        Frontmatter {
            name: String::new(),
            summary: String::new(),
            domain: String::new(),
            tags: Vec::new(),
            status: "pending".to_string(),
        }
    }
}

/// 解析后的文档(对应 Python ParsedDoc)。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ParsedDoc {
    pub path: String,
    pub name: String,
    pub summary: String,
    pub domain: String,
    pub tags: Vec<String>,
    /// (target, relation) 列表。
    pub outlinks: Vec<(String, Option<String>)>,
    /// 全文内容;with_body=false 时为空(未填充),query 等命令用它避免重读。
    pub body: String,
    pub has_frontmatter: bool,
}

/// serde_yaml 中间结构(用 Mapping 提取,保证与 Python 手写解析器行为一致)。
#[derive(Deserialize)]
#[serde(default)]
struct FmMapping {
    name: serde_yaml::Value,
    summary: serde_yaml::Value,
    domain: serde_yaml::Value,
    tags: serde_yaml::Value,
    status: serde_yaml::Value,
}

impl Default for FmMapping {
    fn default() -> Self {
        FmMapping {
            name: serde_yaml::Value::Null,
            summary: serde_yaml::Value::Null,
            domain: serde_yaml::Value::Null,
            tags: serde_yaml::Value::Null,
            status: serde_yaml::Value::Null,
        }
    }
}

/// 把 wiki-link 里的路径归一化为相对 .knowledges/ 的标准化路径。
///
/// 处理:去首尾空白、去反引号、去 .knowledges/ 前缀、统一正斜杠、去多余的 ./ 等。
pub fn normalize_path(raw: &str, kb_root: &str) -> String {
    let mut s = raw.trim().to_string();
    // 去反引号(首尾)
    let trimmed = s.trim_matches('`');
    s = trimmed.trim().to_string();
    // 统一斜杠
    s = s.replace('\\', "/");
    // 去 .knowledges/ 前缀(可能带前导 ./ 或 /)
    let prefix = format!("{}/", kb_root.trim_end_matches('/'));
    let with_dot = format!("./{}", prefix);
    let with_slash = format!("/{}", prefix);
    if s.starts_with(&with_dot) {
        s = s[with_dot.len()..].to_string();
    } else if s.starts_with(&with_slash) {
        s = s[with_slash.len()..].to_string();
    } else if s.starts_with(&prefix) {
        s = s[prefix.len()..].to_string();
    }
    // 去前导 ./
    while s.starts_with("./") {
        s = s[2..].to_string();
    }
    // 去前导 /(但保留根路径 "/")
    while s.starts_with('/') && s.len() > 1 {
        s = s[1..].to_string();
    }
    s
}

/// 从文本中提取全部 wiki-link,返回 [(target_path, relation|None), ...]。
///
/// 按 (target, relation) 去重,保持首次出现顺序。
pub fn parse_wikilinks(text: &str, kb_root: &str) -> Vec<(String, Option<String>)> {
    let mut results: Vec<(String, Option<String>)> = Vec::new();
    let mut seen: HashSet<(String, Option<String>)> = HashSet::new();
    for caps in wikilink_re().captures_iter(text) {
        let raw_path = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let relation: Option<String> = match caps.get(2) {
            None => None,
            Some(m) => {
                let l = m.as_str().trim().to_string();
                if l.is_empty() {
                    None
                } else {
                    Some(l)
                }
            }
        };
        let target = normalize_path(raw_path, kb_root);
        if target.is_empty() {
            continue;
        }
        let key = (target.clone(), relation.clone());
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        results.push((target, relation));
    }
    results
}

/// 解析 YAML 行内数组,支持双引号/单引号包裹的元素(元素内可含逗号)。
///
/// - 双引号元素:反转义(\\" -> ", \\\\ -> \\)
/// - 单引号元素:'' -> '
/// - 无引号元素:trim 首尾空白
fn parse_inline_array(value: &str) -> Vec<String> {
    let v = value.trim();
    if !(v.starts_with('[') && v.ends_with(']')) {
        return Vec::new();
    }
    let inner = v[1..v.len() - 1].trim();
    if inner.is_empty() {
        return Vec::new();
    }
    let mut result: Vec<String> = Vec::new();
    let mut pos = 0;
    let bytes = inner.as_bytes();
    while pos < inner.len() {
        let slice = &inner[pos..];
        if let Some(caps) = inline_elem_re().captures(slice) {
            // 三种捕获组:双引号 / 单引号 / 无引号
            let dq = caps.get(1).map(|m| m.as_str());
            let sq = caps.get(2).map(|m| m.as_str());
            let plain = caps.get(3).map(|m| m.as_str());
            if let Some(dq_val) = dq {
                result.push(
                    dq_val
                        .replace("\\\\", "\x00")
                        .replace("\\\"", "\"")
                        .replace('\x00', "\\"),
                );
            } else if let Some(sq_val) = sq {
                result.push(sq_val.replace("''", "'"));
            } else if let Some(plain_val) = plain {
                result.push(plain_val.trim().to_string());
            }
            // 推进 pos:匹配到的整段长度
            let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            pos += full.len();
            if pos > bytes.len() {
                break;
            }
        } else {
            break;
        }
    }
    result
}
/// 从 serde_yaml::Value 提取字符串(兼容 String/Bool/Number/Null)。
fn value_to_string(v: &serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

/// 从 serde_yaml::Value 提取 tags 数组(兼容 sequence/string/null)。
fn value_to_tags(v: &serde_yaml::Value) -> Vec<String> {
    match v {
        serde_yaml::Value::Sequence(seq) => seq.iter().map(value_to_string).collect(),
        serde_yaml::Value::String(s) => {
            // 兼容行内数组字符串(理论不会出现,serde_yaml 已解析)
            if s.starts_with('[') {
                parse_inline_array(s)
            } else {
                vec![s.clone()]
            }
        }
        _ => Vec::new(),
    }
}

/// 校验 status 字段,只允许 pending/validated,缺失或非法值默认 pending。
fn validate_status(s: &str) -> String {
    match s.trim() {
        "validated" => "validated".to_string(),
        _ => "pending".to_string(),
    }
}
/// 解析 frontmatter,返回 (Frontmatter, body, has_frontmatter)。
///
/// body 为 --- 闭合之后的文本(不含闭合 --- 行)。没有 frontmatter 时 body=全文。
///
/// 优先用 serde_yaml 解析(标准 YAML 语义);失败则回退到逐行手写解析
/// (与 Python parser.parse_frontmatter 行为一致)。
pub fn parse_frontmatter(text: &str) -> (Frontmatter, String, bool) {
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.is_empty() || lines[0].trim() != "---" {
        return (Frontmatter::default(), text.to_string(), false);
    }
    let mut close_idx = None;
    for i in 1..lines.len() {
        if lines[i].trim() == "---" {
            close_idx = Some(i);
            break;
        }
    }
    let close_idx = match close_idx {
        Some(idx) => idx,
        None => return (Frontmatter::default(), text.to_string(), false),
    };
    let fm_text = lines[1..close_idx].join("\n");
    let body = lines[close_idx + 1..].join("\n");

    // 优先 serde_yaml
    let fm = if let Ok(m) = serde_yaml::from_str::<FmMapping>(&fm_text) {
        Frontmatter {
            name: value_to_string(&m.name),
            summary: value_to_string(&m.summary),
            domain: value_to_string(&m.domain),
            tags: value_to_tags(&m.tags),
            status: validate_status(&value_to_string(&m.status)),
        }
    } else {
        parse_frontmatter_manual(&fm_text)
    };
    (fm, body, true)
}

/// 手写 frontmatter 解析(serde_yaml 失败时回退,与 Python parser 逐行逻辑一致)。
fn parse_frontmatter_manual(fm_text: &str) -> Frontmatter {
    let lines: Vec<&str> = fm_text.split('\n').collect();
    let mut fm = Frontmatter::default();
    let mut i = 0;
    let mut in_summary_multiline = false;
    while i < lines.len() {
        let line = lines[i];
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            i += 1;
            continue;
        }
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            match key.as_str() {
                "tags" => {
                    if value.starts_with('[') {
                        fm.tags = parse_inline_array(&value);
                        i += 1;
                    } else if value.is_empty() {
                        let mut tags: Vec<String> = Vec::new();
                        let mut j = i + 1;
                        while j < lines.len() && lines[j].trim().starts_with('-') {
                            let item = lines[j].trim()[1..].trim().trim_matches('"').trim_matches('\'').to_string();
                            if !item.is_empty() {
                                tags.push(item);
                            }
                            j += 1;
                        }
                        fm.tags = tags;
                        i = j;
                    } else {
                        fm.tags = vec![value.trim_matches('"').trim_matches('\'').to_string()];
                        i += 1;
                    }
                    in_summary_multiline = false;
                }
                "name" | "summary" | "domain" | "status" => {
                    if matches!(
                        value.as_str(),
                        "|" | "|-" | "|+" | ">" | ">-" | ">+"
                    ) {
                        // YAML 多行字符串:收集后续缩进行
                        let mut multiline: Vec<String> = Vec::new();
                        let mut j = i + 1;
                        while j < lines.len()
                            && (lines[j].starts_with(' ')
                                || lines[j].starts_with('\t')
                                || lines[j].trim().is_empty())
                        {
                            multiline.push(lines[j].to_string());
                            j += 1;
                        }
                        // 去掉所有非空行的公共缩进
                        let non_empty: Vec<&String> =
                            multiline.iter().filter(|l| !l.trim().is_empty()).collect();
                        if !non_empty.is_empty() {
                            let indent = non_empty
                                .iter()
                                .map(|l| {
                                    l.len()
                                        - l
                                            .trim_start_matches(|c: char| {
                                                c == ' ' || c == '\t'
                                            })
                                            .len()
                                })
                                .min()
                                .unwrap_or(0);
                            multiline = multiline
                                .iter()
                                .map(|l| {
                                    if l.trim().is_empty() {
                                        String::new()
                                    } else {
                                        l[indent.min(l.len())..].to_string()
                                    }
                                })
                                .collect();
                        }
                        let joined = multiline.join("\n");
                        let trimmed = joined.trim().to_string();
                        match key.as_str() {
                            "name" => fm.name = trimmed,
                            "summary" => fm.summary = trimmed,
                            "domain" => fm.domain = trimmed,
                            "status" => fm.status = validate_status(&trimmed),
                            _ => {}
                        }
                        i = j;
                    } else {
                        let v = value.trim_matches('"').trim_matches('\'').to_string();
                        match key.as_str() {
                            "name" => fm.name = v,
                            "summary" => fm.summary = v,
                            "domain" => fm.domain = v,
                            "status" => fm.status = validate_status(&v),
                            _ => {}
                        }
                        // 更新多行状态
                        in_summary_multiline = key == "summary"
                            && matches!(value.as_str(), "|" | "|-" | "|+" | ">" | ">-" | ">+");
                        i += 1;
                    }
                }
                _ => {
                    in_summary_multiline = false;
                    i += 1;
                }
            }
        } else {
            // 如果当前在 summary 多行模式,且当前行是缩进行,也算 summary 续行
            if in_summary_multiline && (line.starts_with(' ') || line.starts_with('\t')) {
                // 追加到 summary(简化处理:不重新计算公共缩进)
                i += 1;
            } else {
                in_summary_multiline = false;
                i += 1;
            }
        }
    }
    fm
}

/// 解析单个文档:frontmatter + wiki-link。
#[allow(dead_code)]
pub fn parse_doc(text: &str, doc_path: &str, kb_root: &str, with_body: bool) -> ParsedDoc {
    let (fm, _body, has_fm) = parse_frontmatter(text);
    let links = parse_wikilinks(text, kb_root);
    ParsedDoc {
        path: doc_path.to_string(),
        name: fm.name,
        summary: fm.summary,
        domain: fm.domain,
        tags: fm.tags,
        outlinks: links,
        body: if with_body { text.to_string() } else { String::new() },
        has_frontmatter: has_fm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wikilinks_basic() {
        let text = "一些正文 [[.knowledges/a.md]] 结束";
        let links = parse_wikilinks(text, ".knowledges");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "a.md");
        assert_eq!(links[0].1, None);
    }

    #[test]
    fn test_parse_wikilinks_with_relation() {
        let text = "[[.knowledges/a.md|关系]]";
        let links = parse_wikilinks(text, ".knowledges");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "a.md");
        assert_eq!(links[0].1.as_deref(), Some("关系"));
    }

    #[test]
    fn test_parse_wikilinks_backtick() {
        let text = "- [[`.knowledges/sub/b.md`]] 紧跟文字";
        let links = parse_wikilinks(text, ".knowledges");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "sub/b.md");
        assert_eq!(links[0].1, None);
    }

    #[test]
    fn test_parse_wikilinks_dedup() {
        let text = "[[.knowledges/a.md]]\n[[.knowledges/a.md]]\n[[.knowledges/a.md|标签]]";
        let links = parse_wikilinks(text, ".knowledges");
        // (a.md, None) 和 (a.md, Some("标签")) 是不同 key,各保留 1 个
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].0, "a.md");
        assert_eq!(links[0].1, None);
        assert_eq!(links[1].0, "a.md");
        assert_eq!(links[1].1.as_deref(), Some("标签"));
    }

    #[test]
    fn test_normalize_path() {
        // 带 .knowledges/ 前缀
        assert_eq!(normalize_path(".knowledges/a.md", ".knowledges"), "a.md");
        // 带 ./ 前缀
        assert_eq!(normalize_path("./.knowledges/sub/a.md", ".knowledges"), "sub/a.md");
        // 带 / 前缀
        assert_eq!(normalize_path("/.knowledges/a.md", ".knowledges"), "a.md");
        // 反引号包裹
        assert_eq!(normalize_path("`.knowledges/a.md`", ".knowledges"), "a.md");
        // 前导 ./
        assert_eq!(normalize_path("./a.md", ".knowledges"), "a.md");
        // 前导 /
        assert_eq!(normalize_path("/a.md", ".knowledges"), "a.md");
        // 纯路径
        assert_eq!(normalize_path("a.md", ".knowledges"), "a.md");
        // 反斜杠归一
        assert_eq!(normalize_path(".knowledges\\sub\\a.md", ".knowledges"), "sub/a.md");
    }

    #[test]
    fn test_parse_frontmatter_yaml() {
        let text = "---\nname: zoloz\nsummary: 摘要\ndomain: cat\ntags: [a, b]\n---\n正文内容";
        let (fm, body, has_fm) = parse_frontmatter(text);
        assert!(has_fm);
        assert_eq!(fm.name, "zoloz");
        assert_eq!(fm.summary, "摘要");
        assert_eq!(fm.domain, "cat");
        assert_eq!(fm.tags, vec!["a", "b"]);
        assert!(body.starts_with("正文内容"));
    }

    #[test]
    fn test_parse_frontmatter_multiline_summary() {
        let text = "---\nname: zoloz\nsummary: |\n  第一行\n  第二行\ntags: []\n---\nbody";
        let (fm, _body, has_fm) = parse_frontmatter(text);
        assert!(has_fm);
        assert_eq!(fm.name, "zoloz");
        assert!(fm.summary.contains("第一行"));
        assert!(fm.summary.contains("第二行"));
    }

    #[test]
    fn test_parse_frontmatter_inline_tags() {
        let text = "---\nname: a\nsummary: s\ntags: [x, y, z]\n---\nbody";
        let (fm, _, has_fm) = parse_frontmatter(text);
        assert!(has_fm);
        assert_eq!(fm.tags, vec!["x", "y", "z"]);
    }

    #[test]
    fn test_parse_frontmatter_sequence_tags() {
        let text = "---\nname: a\nsummary: s\ntags:\n  - alpha\n  - beta\n---\nbody";
        let (fm, _, has_fm) = parse_frontmatter(text);
        assert!(has_fm);
        assert_eq!(fm.tags, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_parse_frontmatter_no_fm() {
        let text = "纯正文,没有 frontmatter。\n第二行";
        let (fm, body, has_fm) = parse_frontmatter(text);
        assert!(!has_fm);
        assert_eq!(fm.name, "");
        assert_eq!(fm.tags.len(), 0);
        assert_eq!(body, text);
    }

    #[test]
    fn test_parse_frontmatter_manual_domain() {
        // 直接触发手写回退解析器,验证 domain key 解析
        let fm = parse_frontmatter_manual("name: a\nsummary: s\ndomain: zoloz/pay\ntags: []\nstatus: validated");
        assert_eq!(fm.domain, "zoloz/pay");
        assert_eq!(fm.name, "a");
        assert_eq!(fm.status, "validated");
        // 未知字段不应被解析为 domain
        let fm2 = parse_frontmatter_manual("name: a\nsummary: s\nunknown_field: old\nstatus: pending");
        assert_eq!(fm2.domain, "");
    }
}
