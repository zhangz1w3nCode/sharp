import { useEffect, useMemo, useRef, useState } from "react";
import { Settings, Link2 } from "lucide-react";
import { useApp } from "../lib/store";
import { graphFrom } from "../lib/derive";
import EmptyDots from "../components/EmptyDots";

interface SimNode {
  x: number;
  y: number;
  vx: number;
  vy: number;
  fx: number | null;
  fy: number | null;
  baseR: number;
  deg: number;
}

interface Pop {
  x: number;
  y: number;
  title: string;
  summary: string;
  deg: number;
}

/** 知识图谱 · force-directed(移植 graph/graph.html 物理模拟,数据由 wikilink 推导) */
export default function GraphView() {
  const { state, api } = useApp();
  const data = useMemo(() => graphFrom(state.cards), [state.cards]);

  const wrapRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const edgesRef = useRef<SVGGElement>(null);
  const nodesRef = useRef<SVGGElement>(null);
  const [size, setSize] = useState({ w: 900, h: 580 });
  const [showSettings, setShowSettings] = useState(false);
  const [arrows, setArrows] = useState(true);
  const [pop, setPop] = useState<Pop | null>(null);

  const sim = useRef({
    nodes: [] as SimNode[],
    raf: 0,
    frames: 0,
    drag: -1,
    panning: false,
    lx: 0,
    ly: 0,
    zoom: 1,
    panX: 0,
    panY: 0,
    moved: false,
    centerF: 0.012,
    repelK: 5200,
    linkK: 0.025,
    linkDist: 120,
    textFade: 0,
    sizeMul: 1,
    thickMul: 1,
    /* tick 循环每帧从这里读最新值,避免闭包捕获首次渲染的 cx/cy/edges(stale closure) */
    cx: 450,
    cy: 290,
    edges: [] as { s: number; t: number; rel: string }[],
  });

  /* 容器尺寸 */
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setSize({ w: el.clientWidth, h: el.clientHeight }));
    ro.observe(el);
    setSize({ w: el.clientWidth, h: el.clientHeight });
    return () => ro.disconnect();
  }, []);

  const cx = size.w / 2;
  const cy = size.h / 2;
  /* 每帧渲染同步最新中心与边集 → tick 循环不会用到过期的闭包值(容器 resize 后图谱仍然居中) */
  sim.current.cx = cx;
  sim.current.cy = cy;
  sim.current.edges = data.edges;

  /* 仅数据变化 → 重建模拟节点(resize 不重置已稳定的布局) */
  useEffect(() => {
    const s = sim.current;
    s.nodes = data.nodes.map((_n, i) => {
      const inDeg = data.edges.filter((e) => e.t === i).length;
      const a = (i / data.nodes.length) * 2 * Math.PI - Math.PI / 2;
      return {
        x: s.cx + Math.cos(a) * 180,
        y: s.cy + Math.sin(a) * 150,
        vx: 0,
        vy: 0,
        fx: null,
        fy: null,
        baseR: 10 + Math.min(inDeg, 5) * 3.5,
        deg: inDeg + data.edges.filter((e) => e.s === i).length,
      };
    });
    /* 先同步定位一次,保证首帧不空 */
    requestAnimationFrame(() => {
      render();
      runSim();
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data]);

  function updateRadii() {
    const s = sim.current;
    const ng = nodesRef.current;
    if (!ng) return;
    s.nodes.forEach((n, i) => {
      const g = ng.children[i] as SVGGElement | undefined;
      if (!g) return;
      g.querySelector("circle")?.setAttribute("r", String(n.baseR * s.sizeMul));
      g.querySelector("text")?.setAttribute("y", String(n.baseR * s.sizeMul + 4));
    });
  }

  function render() {
    const s = sim.current;
    const eg = edgesRef.current;
    const ng = nodesRef.current;
    if (!eg || !ng) return;
    s.edges.forEach((e, i) => {
      const a = s.nodes[e.s];
      const b = s.nodes[e.t];
      if (!a || !b) return;
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const d = Math.sqrt(dx * dx + dy * dy) || 1;
      const ux = dx / d;
      const uy = dy / d;
      const l = eg.children[i] as SVGLineElement | undefined;
      if (!l) return;
      l.setAttribute("x1", String(a.x + ux * a.baseR * s.sizeMul));
      l.setAttribute("y1", String(a.y + uy * a.baseR * s.sizeMul));
      l.setAttribute("x2", String(b.x - ux * (b.baseR * s.sizeMul + 4)));
      l.setAttribute("y2", String(b.y - uy * (b.baseR * s.sizeMul + 4)));
    });
    s.nodes.forEach((n, i) => {
      const g = ng.children[i] as SVGGElement | undefined;
      if (g) g.setAttribute("transform", `translate(${n.x},${n.y})`);
    });
  }

  function tick() {
    const s = sim.current;
    const ns = s.nodes;
    for (let i = 0; i < ns.length; i++) {
      for (let j = i + 1; j < ns.length; j++) {
        const dx = ns[i].x - ns[j].x;
        const dy = ns[i].y - ns[j].y;
        const d2 = dx * dx + dy * dy + 0.01;
        const d = Math.sqrt(d2);
        const f = s.repelK / d2;
        ns[i].vx += (dx / d) * f;
        ns[i].vy += (dy / d) * f;
        ns[j].vx -= (dx / d) * f;
        ns[j].vy -= (dy / d) * f;
      }
    }
    s.edges.forEach((e) => {
      const a = ns[e.s];
      const b = ns[e.t];
      if (!a || !b) return;
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const d = Math.sqrt(dx * dx + dy * dy) + 0.01;
      const f = (d - s.linkDist) * s.linkK;
      a.vx += (dx / d) * f;
      a.vy += (dy / d) * f;
      b.vx -= (dx / d) * f;
      b.vy -= (dy / d) * f;
    });
    let ke = 0;
    ns.forEach((n) => {
      if (n.fx != null && n.fy != null) {
        n.x = n.fx;
        n.y = n.fy;
        n.vx = 0;
        n.vy = 0;
      } else {
        n.vx += (s.cx - n.x) * s.centerF;
        n.vy += (s.cy - n.y) * s.centerF;
        n.vx *= 0.84;
        n.vy *= 0.84;
        const sp = Math.sqrt(n.vx * n.vx + n.vy * n.vy);
        if (sp > 18) {
          n.vx = (n.vx / sp) * 18;
          n.vy = (n.vy / sp) * 18;
        }
        n.x += n.vx;
        n.y += n.vy;
      }
      ke += n.vx * n.vx + n.vy * n.vy;
    });
    render();
    s.frames++;
    if ((s.frames < 260 && ke > 0.35) || s.drag >= 0) s.raf = requestAnimationFrame(tick);
    else s.raf = 0;
  }

  function runSim() {
    const s = sim.current;
    s.frames = 0;
    if (!s.raf) s.raf = requestAnimationFrame(tick);
  }

  useEffect(() => {
    runSim();
    return () => {
      /* StrictMode 双挂载:cleanup 取消后必须归零,否则第二次 runSim 误判循环仍在跑 */
      cancelAnimationFrame(sim.current.raf);
      sim.current.raf = 0;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /* 视口变换 */
  function applyVP() {
    const s = sim.current;
    const vp = svgRef.current?.querySelector("#g-vp");
    if (vp) vp.setAttribute("transform", `translate(${s.panX},${s.panY}) scale(${s.zoom})`);
  }
  function toGraph(sx: number, sy: number) {
    const s = sim.current;
    const r = svgRef.current!.getBoundingClientRect();
    return { x: ((sx - r.left) * (size.w / r.width) - s.panX) / s.zoom, y: ((sy - r.top) * (size.h / r.height) - s.panY) / s.zoom };
  }

  useEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const s = sim.current;
      const f = e.deltaY < 0 ? 1.1 : 0.9;
      const nz = Math.max(0.4, Math.min(2.5, s.zoom * f));
      const gx = (size.w / 2 - s.panX) / s.zoom;
      const gy = (size.h / 2 - s.panY) / s.zoom;
      s.zoom = nz;
      s.panX = size.w / 2 - gx * s.zoom;
      s.panY = size.h / 2 - gy * s.zoom;
      applyVP();
    };
    svg.addEventListener("wheel", onWheel, { passive: false });
    return () => svg.removeEventListener("wheel", onWheel);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [size]);

  function onMouseDown(e: React.MouseEvent) {
    const s = sim.current;
    const g = (e.target as Element).closest(".g-node");
    if (g) {
      const i = Number(g.getAttribute("data-i"));
      s.drag = i;
      s.moved = false;
      const p = toGraph(e.clientX, e.clientY);
      s.nodes[i].fx = p.x;
      s.nodes[i].fy = p.y;
      runSim();
      svgRef.current?.classList.add("dragging");
      e.preventDefault();
      return;
    }
    s.panning = true;
    s.lx = e.clientX;
    s.ly = e.clientY;
    svgRef.current?.classList.add("panning");
  }

  useEffect(() => {
    const mv = (e: MouseEvent) => {
      const s = sim.current;
      if (s.drag >= 0) {
        const p = toGraph(e.clientX, e.clientY);
        s.nodes[s.drag].fx = p.x;
        s.nodes[s.drag].fy = p.y;
        s.moved = true;
        return;
      }
      if (!s.panning) return;
      const r = svgRef.current!.getBoundingClientRect();
      s.panX += (e.clientX - s.lx) * (size.w / r.width);
      s.panY += (e.clientY - s.ly) * (size.h / r.height);
      s.lx = e.clientX;
      s.ly = e.clientY;
      applyVP();
    };
    const up = () => {
      const s = sim.current;
      if (s.drag >= 0) {
        s.nodes[s.drag].fx = null;
        s.nodes[s.drag].fy = null;
        s.drag = -1;
        svgRef.current?.classList.remove("dragging");
        return;
      }
      s.panning = false;
      svgRef.current?.classList.remove("panning");
    };
    window.addEventListener("mousemove", mv);
    window.addEventListener("mouseup", up);
    return () => {
      window.removeEventListener("mousemove", mv);
      window.removeEventListener("mouseup", up);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [size]);

  /* 悬停高亮 + 预览卡 */
  function highlight(i: number) {
    const s = sim.current;
    const ng = nodesRef.current;
    const eg = edgesRef.current;
    if (!ng || !eg) return;
    const nb = new Set<number>();
    data.edges.forEach((e) => {
      if (e.s === i) nb.add(e.t);
      if (e.t === i) nb.add(e.s);
    });
    [...ng.children].forEach((g, j) => {
      g.classList.toggle("dim", j !== i && !nb.has(j));
      g.classList.toggle("lbl", j === i || nb.has(j));
    });
    [...eg.children].forEach((l, k) => {
      const e = data.edges[k];
      l.classList.toggle("hl", e.s === i || e.t === i);
    });
    const n = data.nodes[i];
    const sn = s.nodes[i];
    if (n && sn) {
      const px = sn.x * s.zoom + s.panX;
      const py = sn.y * s.zoom + s.panY;
      let left = px + sn.baseR * s.sizeMul * s.zoom + 10;
      let top = py - 24;
      if (left + 250 > size.w) left = px - sn.baseR * s.sizeMul * s.zoom - 260;
      if (top < 6) top = py + sn.baseR * s.sizeMul * s.zoom + 10;
      setPop({
        x: Math.max(6, Math.min(size.w - 256, left)),
        y: Math.max(6, Math.min(size.h - 90, top)),
        title: n.title,
        summary: n.summary,
        deg: sn.deg,
      });
    }
  }
  function clearHL() {
    const ng = nodesRef.current;
    const eg = edgesRef.current;
    if (ng) [...ng.children].forEach((g) => g.classList.remove("dim", "lbl"));
    if (eg) [...eg.children].forEach((l) => l.classList.remove("hl"));
    setPop(null);
  }

  function clickNode(i: number) {
    if (sim.current.moved) {
      sim.current.moved = false;
      return;
    }
    const n = data.nodes[i];
    if (!n) return;
    const c = state.cards.find((x) => x.path === n.id);
    if (c) {
      api.setView("search");
      api.setSearchDetail(c.id);
    } else {
      api.openSheet({ kind: "broken", path: n.id });
    }
  }

  function slider(label: string, min: number, max: number, init: number, fn: (v: number) => void) {
    return (
      <div className="g-set-row" key={label}>
        <span>{label}</span>
        <input
          type="range"
          min={min}
          max={max}
          defaultValue={init}
          onChange={(e) => {
            fn(Number(e.target.value));
            const v = e.target.parentElement?.querySelector(".val");
            if (v) v.textContent = e.target.value;
          }}
        />
        <span className="val">{init}</span>
      </div>
    );
  }

  if (state.cards.length === 0) {
    return (
      <main className="pane pane-main">
        <EmptyDots hint="暂无卡片,图谱为空" />
      </main>
    );
  }

  return (
    <main className="pane pane-main" style={{ background: "transparent", padding: 0 }}>
      <div className="graph-wrap" ref={wrapRef}>
        <svg ref={svgRef} viewBox={`0 0 ${size.w} ${size.h}`} preserveAspectRatio="xMidYMid meet" onMouseDown={onMouseDown}>
          <defs>
            <marker id="g-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto">
              <path d="M0,0 L10,5 L0,10 z" fill="#8e8b82" fillOpacity=".5" />
            </marker>
            <marker id="g-arrow-hl" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto">
              <path d="M0,0 L10,5 L0,10 z" fill="#cc785c" />
            </marker>
          </defs>
          <g id="g-vp">
            <g ref={edgesRef} className={`g-edges${arrows ? "" : " no-arrows"}`}>
              {data.edges.map((_, i) => (
                <line key={i} className="g-edge" />
              ))}
            </g>
            <g ref={nodesRef}>
              {data.nodes.map((n, i) => (
                <g
                  key={n.id}
                  data-i={i}
                  className={`g-node${n.unresolved ? " unresolved" : ""}`}
                  onMouseEnter={() => highlight(i)}
                  onMouseLeave={clearHL}
                  onClick={(e) => {
                    e.stopPropagation();
                    clickNode(i);
                  }}
                >
                  <circle r={(10 + Math.min(data.edges.filter((e) => e.t === i).length, 5) * 3.5) * sim.current.sizeMul} />
                  <text y={(10 + Math.min(data.edges.filter((e) => e.t === i).length, 5) * 3.5) * sim.current.sizeMul + 4}>{n.title}</text>
                </g>
              ))}
            </g>
          </g>
        </svg>

        <button
          className={`g-cog${showSettings ? " on" : ""}`}
          title="图谱设置"
          onClick={() => setShowSettings((v) => !v)}
        >
          <Settings size={16} strokeWidth={1.7} />
        </button>

        {showSettings && (
          <div className="g-settings">
            <h4>力学</h4>
            {slider("向心", 0, 100, 12, (v) => {
              sim.current.centerF = v / 1000;
              runSim();
            })}
            {slider("斥力", 0, 100, 52, (v) => {
              sim.current.repelK = v * 100;
              runSim();
            })}
            {slider("引力", 0, 100, 25, (v) => {
              sim.current.linkK = v / 1000;
              runSim();
            })}
            {slider("连线距离", 40, 240, 120, (v) => {
              sim.current.linkDist = v;
              runSim();
            })}
            <h4>显示</h4>
            <div
              className={`g-toggle${arrows ? " on" : ""}`}
              onClick={() => setArrows((a) => !a)}
            >
              <span className="sw" />
              箭头
            </div>
            {slider("标签淡出", 0, 100, 0, (v) => {
              sim.current.textFade = v / 100;
              svgRef.current?.style.setProperty("--text-fade", String(v / 100));
            })}
            {slider("节点大小", 50, 200, 100, (v) => {
              sim.current.sizeMul = v / 100;
              updateRadii();
              render();
            })}
            {slider("连线粗细", 50, 300, 100, (v) => {
              sim.current.thickMul = v / 100;
              svgRef.current?.style.setProperty("--link-thickness", `${(v / 100) * 1.2}px`);
            })}
          </div>
        )}

        {pop && (
          <div className="g-pop" style={{ left: pop.x, top: pop.y }}>
            <div className="g-pop-t">{pop.title}</div>
            <div className="g-pop-s">{pop.summary}</div>
            <div className="g-pop-m">
              <Link2 size={11} strokeWidth={2} />
              链接 · {pop.deg}
            </div>
          </div>
        )}
      </div>
    </main>
  );
}
