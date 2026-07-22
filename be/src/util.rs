//! util.rs - 公共输出工具(消除各模块重复定义)。

use serde_json::{json, Value};

pub(crate) fn output_json(data: &Value) {
    match serde_json::to_string_pretty(data) {
        Ok(s) => println!("{s}"),
        Err(e) => {
            // 错误分支也必须产出合法 JSON(下游 jq/解析器不崩);
            // 用 json! 构造确保特殊字符被转义(手拼字符串会有此风险)。
            let err = json!({"error": format!("output serialization failed: {e}")});
            eprintln!(
                "{}",
                serde_json::to_string(&err)
                    .unwrap_or_else(|_| "{\"error\":\"output serialization failed\"}".to_string())
            );
        }
    }
}

pub(crate) fn error_json(msg: &str) -> i32 {
    output_json(&json!({"error": msg}));
    1
}

pub(crate) fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}
