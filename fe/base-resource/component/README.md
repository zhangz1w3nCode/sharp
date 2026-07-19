# base-resource · 设计令牌与组件库

Claude design 风格的**单一源**。原型图与未来实现均引用此处,不再内联 hex。

- 令牌:`./tokens.css`(colors / typography / radius / spacing / elevation)
- 组件:`./components.css`(可复用桌面端组件类)
- 设计依据:`../theme/DESIGN.md`

## 用法

每个页面 `<head>` 引入两份(路径按页面位置调整):

```html
<link rel="stylesheet" href="../../base-resource/component/tokens.css">
<link rel="stylesheet" href="../../base-resource/component/components.css">
```

页面 `<style>` 只保留**页面专属** CSS(窗口状态、布局、特定渲染),**不再重复** `:root` 令牌与通用组件类。

## 令牌速查(tokens.css)

| 类别 | 令牌 |
|---|---|
| 品牌 | `--primary` `#cc785c` · `--primary-active` `#a9583e` |
| 文字 | `--ink` `#141413` · `--body` `#3d3d3a` · `--body-strong` · `--muted` · `--muted-soft` |
| 面 | `--canvas` `#faf9f5` · `--surface-soft` `#f5f0e8` · `--surface-card` `#efe9de` · `--surface-cream-strong` `#e8e0d2` |
| 暗面 | `--surface-dark` `#181715` · `--surface-dark-elevated` · `--surface-dark-soft` · `--on-dark` · `--on-dark-soft` |
| 语义 | `--accent-teal` `--accent-amber` `--success` `--warning` `--error` |
| 线 | `--hairline` `#e6dfd8` · `--hairline-soft` · `--hairline-dark` |
| 字体 | `--serif`(Cormorant/Tiempos)· `--sans`(StyreneB/Inter)· `--mono`(JetBrains Mono) |
| 圆角 | `--r-xs`4 `--r-sm`6 `--r-md`8 `--r-lg`12 `--r-xl`16 `--r-pill` |
| 间距 | `--s-xxs`4 … `--s-lg`24 `--s-xl`32 `--s-xxl`48 `--s-section`96 |
| 高度 | `--shadow-window` `--shadow-float` `--shadow-modal` `--ring-coral` `--tint-coral` |

**铁律**:组件内一律 `var(--…)`,禁止内联 hex。珊瑚稀缺(主 CTA + 焦点环 + 内联链接)。serif 标题 weight 400、负字距。

## 组件速查(components.css)

### 页面 / 窗口
- `.desk-floor` / `.desk-floor.center` — 原型桌面底(`--surface-soft`),窗口浮于其上
- `.win` `.win-shadow` — 应用窗口(canvas + 圆角 + OS chrome 阴影)
- `.lights` `.light.red/.yellow/.green` — macOS 交通灯(hover 显形)
- `.resizer` — 可拖拽分隔(珊瑚 tint hover)

### 排版
- `.t-title` — serif 页标题(500/-0.02em)
- `.t-display` — serif 显示标题(400/-0.3px)
- `.eyebrow` — 上标签(caption-uppercase,12/500/1.5px,muted-soft)

### 按钮(紧凑 36px)
- `.btn` 基座 + `.btn-primary`(珊瑚)/ `.btn-secondary`(canvas+hairline)/ `.btn-ghost`/ `.btn-secondary-on-dark`/ `.btn-text-link`
- `.btn-icon` 配 `.btn` 变体 = 方形图标按钮;`.btn-icon-sm` 30px
- `.tb` — 工具栏幽灵图标(28px)

### 输入
- `.text-input` — canvas + hairline,聚焦 = 珊瑚边 + `--ring-coral`
- `.text-input.textarea` — 多行
- `.title-input` — serif 大标题输入(透明,下划线 → 珊瑚聚焦)

### 卡片 / 面
- `.card` — 知识库卡:canvas 浮于 surface-card,hover/selected = cream-strong + hairline(无浮起)
- `.surface-card` — 通用内容卡(surface-card,圆角 lg,padding xl)
- `.panel` — 阅读面板(canvas + hairline)
- `.code-window` — 暗色代码窗(surface-dark-soft,mono);子类 `.k`(teal)/`.s`(amber)/`.c`(muted)/`.h`(on-dark)/`.lk`(珊瑚)

### 导航
- `.entry` + `.entry.active`/`.focused` — 侧栏条目(选中=cream-strong+珊瑚圆点,hover=surface-soft,无缩放)
- `.entry .dot` — 圆点

### 状态 / 内联
- `.dot` `.dot.muted/.success/.error` — 状态圆点
- `.wl` — wikilink 珊瑚内链;`.wl.broken` — 断链虚线
- `.text-link` — 珊瑚文本链
- `.star` / `.star.on` — 收藏星(珊瑚填充)

### 标签 / 键
- `.badge-pill` `.badge-coral` `.badge`
- `.kbd` / `.kbd-light`

### 反馈
- `.toast` — 暗色提示丸(`.show` 显)
- `.overlay` + `.sheet` + `.sheet-title` — macOS sheet(顶部滑入 + 背景模糊)
- `.popover` — 浮动 canvas 卡 + hairline + `--shadow-float`

### 杂项
- `.empty-dots` — 三点空状态 logo
- `.mark` — Anthropic 径向 spike 品牌符(fill 继承珊瑚)

## 设计约束(摘要,详见 DESIGN.md)

- 画布 = 暖奶油,禁纯白/冷灰
- serif 标题 weight 400 + 负字距;sans 正文 400;mono 代码
- 珊瑚仅用于:主 CTA、焦点环、内联链接、品牌符(稀缺)
- 三表面三色:cream / cream-card / dark-navy,不引第四色
- color-block 分层 + hairline;阴影罕用(窗口/浮动对象)
- 悬停无 scale/lift,仅色变(主按钮 press 加深)
- 圆角分级:按钮/输入 8、卡片 12、hero 16、badge pill

## v2 复用模式

原型页改造步骤:
1. `<head>` 加两份 `<link>`(tokens.css + components.css)
2. 删除页内 `:root` 令牌块(令牌已集中)
3. 删除与 `components.css` 重复的组件类定义,保留页面专属(窗口状态、布局、特定子元素)
4. 标记/按钮/卡片等改用组件类
