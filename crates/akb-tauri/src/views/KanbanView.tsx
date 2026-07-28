import { useMemo } from "react";

import { useApp } from "../lib/store";

import type { Card, ReviewItem } from "../lib/types";
interface TrendPoint {
  date: string;
  count: number;
}

interface SourcePoint {
  source: string;
  count: number;
}

interface ActivityItem {
  title: string;
  subtitle: string;
  ts: number;
}

function startOfDay(ts: number) {
  const d = new Date(ts);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

function formatDate(ts: number) {
  const d = new Date(ts);
  return `${d.getMonth() + 1}月${d.getDate()}日`;
}

function formatFull(ts: number) {
  const d = new Date(ts);
  return `${d.getFullYear()}/${d.getMonth() + 1}/${d.getDate()}`;
}

function buildTrend(cards: Card[], days = 14): TrendPoint[] {
  const today = startOfDay(Date.now());
  const map = new Map<number, number>();
  for (let i = days - 1; i >= 0; i--) {
    map.set(today - i * 86400000, 0);
  }
  cards.forEach((c) => {
    const day = startOfDay(c.updatedAt);
    if (map.has(day)) {
      map.set(day, (map.get(day) ?? 0) + 1);
    }
  });
  return Array.from(map.entries())
    .sort(([a], [b]) => a - b)
    .map(([ts, count]) => ({ date: formatDate(ts), count }));
}

function buildSources(queue: ReviewItem[]): SourcePoint[] {
  const map = new Map<string, number>();
  queue.forEach((item) => {
    map.set(item.source, (map.get(item.source) ?? 0) + 1);
  });
  return Array.from(map.entries())
    .sort((a, b) => b[1] - a[1])
    .map(([source, count]) => ({ source, count }));
}

function buildHeatmap(cards: Card[], weeks = 12): number[][] {
  const today = new Date();
  const day = today.getDay();
  const endOfToday = startOfDay(Date.now()) + 86400000;
  const endWeek = endOfToday + (6 - day) * 86400000;
  const buckets = new Map<number, number>();
  for (let w = 0; w < weeks; w++) {
    for (let d = 0; d < 7; d++) {
      const ts = endWeek - (weeks - 1 - w) * 7 * 86400000 + (d - 6) * 86400000;
      buckets.set(ts, 0);
    }
  }
  cards.forEach((c) => {
    const key = startOfDay(c.updatedAt);
    if (buckets.has(key)) {
      buckets.set(key, (buckets.get(key) ?? 0) + 1);
    }
  });
  const rows: number[][] = Array.from({ length: 7 }, () => []);
  for (let w = 0; w < weeks; w++) {
    for (let d = 0; d < 7; d++) {
      const ts = endWeek - (weeks - 1 - w) * 7 * 86400000 + (d - 6) * 86400000;
      rows[d][w] = buckets.get(ts) ?? 0;
    }
  }
  return rows;
}

function recentActivity(cards: Card[], queue: ReviewItem[], limit = 10): ActivityItem[] {
  const items: ActivityItem[] = [
    ...cards.map((c) => ({
      title: c.title,
      subtitle: "已入库卡片",
      ts: c.updatedAt,
    })),
    ...queue.map((q) => ({
      title: q.title,
      subtitle: `待审 · ${q.source}`,
      ts: q.ts,
    })),
  ];
  items.sort((a, b) => b.ts - a.ts);
  return items.slice(0, limit);
}

function LineChart({ data }: { data: TrendPoint[] }) {
  const width = 320;
  const height = 160;
  const padding = 24;
  const innerW = width - padding * 2;
  const innerH = height - padding * 2;
  const max = Math.max(1, ...data.map((d) => d.count));
  const points = data.map((d, i) => {
    const x = padding + (i / Math.max(1, data.length - 1)) * innerW;
    const y = padding + innerH - (d.count / max) * innerH;
    return { x, y, value: d.count };
  });
  const polyline = points.map((p) => `${p.x},${p.y}`).join(" ");
  const ticks = [0, Math.ceil(max / 2), Math.ceil(max)];

  return (
    <svg viewBox={`0 0 ${width} ${height}`} className="dashboard-chart">
      <g>
        {ticks.map((t, i) => {
          const y = padding + innerH - (t / max) * innerH;
          return (
            <g key={i}>
              <line x1={padding} y1={y} x2={width - padding} y2={y} className="chart-grid" />
              <text x={padding - 6} y={y + 3} className="chart-tick" textAnchor="end">
                {t}
              </text>
            </g>
          );
        })}
      </g>
      <polyline fill="none" points={polyline} className="chart-line" />
      {points.map((p, i) => (
        <circle key={i} cx={p.x} cy={p.y} r={3} className="chart-dot" />
      ))}
    </svg>
  );
}

function BarChart({ data }: { data: SourcePoint[] }) {
  const width = 320;
  const height = 160;
  const padding = 24;
  const innerW = width - padding * 2;
  const innerH = height - padding * 2;
  const max = Math.max(1, ...data.map((d) => d.count));
  const barW = data.length > 0 ? (innerW / data.length) * 0.6 : 0;
  const gap = data.length > 0 ? (innerW / data.length) * 0.4 : 0;

  return (
    <svg viewBox={`0 0 ${width} ${height}`} className="dashboard-chart">
      {data.map((d, i) => {
        const h = (d.count / max) * innerH;
        const x = padding + i * (barW + gap) + gap / 2;
        const y = padding + innerH - h;
        return (
          <g key={i}>
            <rect x={x} y={y} width={barW} height={h} rx={4} className="chart-bar" />
            <text x={x + barW / 2} y={y - 6} className="chart-label" textAnchor="middle">
              {d.count}
            </text>
            <text x={x + barW / 2} y={height - 6} className="chart-label" textAnchor="middle">
              {d.source}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

export default function KanbanView() {
  const { state } = useApp();
  const today = startOfDay(Date.now());

  const totalCards = state.cards.length;
  const pendingCount = state.queue.length;
  const todayCount = useMemo(() => {
    const cardToday = state.cards.filter((c) => startOfDay(c.updatedAt) === today).length;
    const queueToday = state.queue.filter((q) => startOfDay(q.ts) === today).length;
    return cardToday + queueToday;
  }, [state.cards, state.queue, today]);

  const trend = useMemo(() => buildTrend(state.cards), [state.cards]);
  const sources = useMemo(() => buildSources(state.queue), [state.queue]);
  const heatmap = useMemo(() => buildHeatmap(state.cards), [state.cards]);
  const activity = useMemo(() => recentActivity(state.cards, state.queue), [state.cards, state.queue]);

  const maxHeat = Math.max(1, ...heatmap.flat());

  return (
    <main className="pane pane-main dashboard-scroll">
      <div className="dashboard">
        <header className="dashboard-header">
          <h1 className="dashboard-title">统计概览</h1>
          <p className="dashboard-subtitle">{formatFull(Date.now())}</p>
        </header>

        <section className="kpi-grid">
          <div className="kpi-card">
            <div className="kpi-value">{totalCards}</div>
            <div className="kpi-label">卡片总数</div>
          </div>
          <div className="kpi-card">
            <div className="kpi-value">{pendingCount}</div>
            <div className="kpi-label">待审数量</div>
          </div>
          <div className="kpi-card">
            <div className="kpi-value">{todayCount}</div>
            <div className="kpi-label">今日新增</div>
          </div>
          <div className="kpi-card">
            <div className="kpi-value">{sources.length}</div>
            <div className="kpi-label">来源类型</div>
          </div>
        </section>

        <section className="dashboard-row">
          <div className="dashboard-card">
            <h2 className="dashboard-card-title">创建趋势（近 14 天）</h2>
            {trend.length > 0 ? <LineChart data={trend} /> : <p className="dashboard-empty">暂无数据</p>}
          </div>
          <div className="dashboard-card">
            <h2 className="dashboard-card-title">来源分布</h2>
            {sources.length > 0 ? <BarChart data={sources} /> : <p className="dashboard-empty">暂无数据</p>}
          </div>
        </section>

        <section className="dashboard-card heatmap-card">
            <h2 className="dashboard-card-title">活跃度热力图（近 12 周）</h2>
            <div className="heatmap-wrap">
              <div className="heatmap">
                {heatmap.map((row, ri) =>
                  row.map((value, ci) => {
                    const ratio = value / maxHeat;
                    const opacity = value === 0 ? 0.15 : 0.25 + ratio * 0.75;
                    return (
                      <div
                        key={`${ri}-${ci}`}
                        className="heatmap-cell"
                        style={{ opacity }}
                        title={`${value} 次更新`}
                      />
                    );
                  })
                )}
              </div>
              <div className="heatmap-legend">
                <span>少</span>
                <span className="heatmap-legend-cell" style={{ opacity: 0.15 }} />
                <span className="heatmap-legend-cell" style={{ opacity: 0.4 }} />
                <span className="heatmap-legend-cell" style={{ opacity: 0.6 }} />
                <span className="heatmap-legend-cell" style={{ opacity: 0.8 }} />
                <span className="heatmap-legend-cell" style={{ opacity: 1 }} />
                <span>多</span>
              </div>
            </div>
        </section>

        <section className="dashboard-card recent-card">
          <h2 className="dashboard-card-title">最近活跃</h2>
          {activity.length > 0 ? (
            <ul className="recent-list">
              {activity.map((item, i) => (
                <li key={i} className="recent-item">
                  <div className="recent-title">{item.title}</div>
                  <div className="recent-subtitle">{item.subtitle}</div>
                  <div className="recent-date">{formatFull(item.ts)}</div>
                </li>
              ))}
            </ul>
          ) : (
            <p className="dashboard-empty">暂无动态</p>
          )}
        </section>
      </div>
    </main>
  );
}
