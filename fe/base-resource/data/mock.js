/* =========================================================================
   base-resource/data/mock.js  —  shared fixtures (extracted from v2)
   cards (home/kb/search) · graph nodes/edges · review queue items
   <script src="../../base-resource/data/mock.js"></script>  →  window.MOCK.*
   ========================================================================= */
(function(){
  const CARDS=[
    {t:"React Server Components 速记",s:"RSC 在服务端渲染、不 hydrate;与 Client Components 边界靠 'use client'。",f:true,
     summary:"RSC 在服务端渲染、不 hydrate;与 Client Components 边界靠 'use client'。",
     backlinks:[{n:"Tauri v2 vibrancy",r:"相关"},{n:"SF Pro 字体回退栈",r:"扩展"}]},
    {t:"Tauri v2 窗口 vibrancy",s:"macOS 用 windowEffects+vibrancy;需 NSVisualEffectView 兼容配置。",f:false,
     summary:"macOS 下窗口毛玻璃的实现要点与 NSVisualEffectView 兼容配置。",
     backlinks:[{n:"macOS HIG 三原则",r:"引用"},{n:"Wikilink 解析",r:"相关"},{n:"未知文档",r:"引用",broken:true}]},
    {t:"Tailwind v4 @theme 令牌",s:"CSS 用 @theme 定义 --color-* 即生成工具类,替代 tailwind.config。",f:true,
     summary:"@theme 定义 --color-* 即生成工具类,替代 tailwind.config。",
     backlinks:[{n:"极简设计:去分隔线",r:"相关"}]},
    {t:"知识图谱 force-directed 布局",s:"d3-force / vis-network;节点按 degree 缩放,边有向。",f:false,
     summary:"d3-force / vis-network;节点按入度放大,边有向。",
     backlinks:[{n:"Wikilink 解析",r:"依赖"}]},
    {t:"Wikilink 解析 [slug|ref]",s:"正则提取 slug 与显示文本;断链虚线 + 创建入口。",f:false,
     summary:"正则提取 path 与 relation;断链虚线 + 创建入口。",
     backlinks:[{n:"Tauri v2 vibrancy",r:"相关"}]},
    {t:"macOS HIG 三原则",s:"Clarity / Deference / Depth;内容优先、UI 克制、层级靠模糊与表面色。",f:true,
     summary:"Clarity(清晰)/ Deference(克制)/ Depth(层次)。",
     backlinks:[{n:"Tauri v2 vibrancy",r:"引用"},{n:"极简设计:去分隔线",r:"引用"}]},
    {t:"SF Pro 字体回退栈",s:"-apple-system 在 macOS 解析为真 SF Pro;跨平台用 Inter 近似。",f:false,
     summary:"-apple-system 在 macOS 解析为真 SF Pro;跨平台用 Inter 近似。",
     backlinks:[]},
    {t:"极简设计:去分隔线",s:"表面色反差 + 间隙分层,不用 hairline,更克制高级。",f:false,
     summary:"表面色反差 + 间隙分层,不用 hairline,更克制高级。",
     backlinks:[{n:"macOS HIG 三原则",r:"引用"}]},
  ];

  const NODES=[
    {id:"tauri",t:"Tauri v2 窗口 vibrancy",s:"macOS vibrancy + NSHUDWindowMaterial 兼容配置。"},
    {id:"hig",t:"macOS HIG 三原则",s:"Clarity / Deference / Depth;内容优先、UI 克制。"},
    {id:"wl",t:"Wikilink 解析 [path|relation]",s:"正则提取 path 与 relation;断链虚线+创建入口。"},
    {id:"sfpro",t:"SF Pro 字体回退栈",s:"-apple-system 在 macOS 解析为真 SF Pro;跨平台用 Inter 近似。"},
    {id:"rsc",t:"React Server Components 速记",s:"RSC 服务端渲染、不 hydrate;边界靠 'use client'。"},
    {id:"tw",t:"Tailwind v4 @theme 令牌",s:"@theme 定义 --color-* 即生成工具类,替代 config。"},
    {id:"graph",t:"知识图谱 force-directed 布局",s:"节点按 degree 缩放;边有向(A→B)。"},
    {id:"nd",t:"极简设计:去分隔线",s:"表面色反差 + 间隙分层,不用 hairline。"},
    {id:"unknown",t:"未知文档",s:"路径未解析;点击可创建。",unresolved:true},
  ];
  const EDGES=[
    {s:0,t:1,r:"引用"},{s:0,t:2,r:"相关"},{s:0,t:3,r:"扩展"},
    {s:4,t:5,r:"相关"},{s:6,t:2,r:"依赖"},{s:7,t:1,r:"引用"},
    {s:1,t:3,r:"相关"},{s:5,t:7,r:"相关"},{s:0,t:8,r:"引用"},
  ];

  const ITEMS=[
    {type:"new",src:"大模型",title:"React Server Components 速记",snippet:"RSC 在服务端渲染、不 hydrate;与 Client Components 边界靠 'use client'。",ts:"2 分钟前",
     full:"## RSC 速记\n\nRSC 在服务端渲染、不 hydrate;与 Client Components 边界靠 'use client'。\n\n- 服务端组件不能 useState/useEffect\n- 客户端组件以 'use client' 声明"},
    {type:"update",src:"大模型",title:"Tauri v2 窗口 vibrancy",snippet:"更新 vibrancy 配置说明,补充 NSHUDWindowMaterial 兼容。",ts:"8 分钟前"},
    {type:"new",src:"手动新建",title:"极简设计:去分隔线",snippet:"表面色反差 + 间隙分层,不用 hairline,更克制高级。",ts:"15 分钟前",
     full:"## 去分隔线\n\n表面色反差(canvas→card→muted)+ 间隙分层,不用 hairline。\n\n- box 隔离:色调反差 + 圆角\n- 无描边、无分割线"},
    {type:"update",src:"大模型",title:"Wikilink 解析 [path|relation]",snippet:"slug 概念改为相对 path;关系为自由文本。",ts:"23 分钟前"},
    {type:"new",src:"大模型",title:"知识图谱 force-directed 布局",snippet:"d3-force / vis-network;节点按 degree 缩放,边有向。",ts:"31 分钟前",
     full:"## force-directed\n\n- d3-force / vis-network\n- 节点按 degree 缩放\n- 边有向(A→B,A 含 [B|…])"},
  ];

  window.MOCK={CARDS,NODES,EDGES,ITEMS};
})();
