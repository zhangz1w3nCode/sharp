/* FileTree.tsx — VS Code 风格文件树（懒加载 · chevron + 纯文字 · 极简） */
import { useEffect, useRef, useState, useCallback } from "react";
import { ChevronRight, ChevronDown } from "lucide-react";
import { useApp } from "../lib/store";
import { scanDir, TreeNode } from "../lib/fs";


interface TreeItemProps {
  node: TreeNode;
  depth: number;
  expanded: Set<string>;
  loaded: Map<string, TreeNode[]>;
  onToggle: (node: TreeNode) => void;
  selectedPath: string | null;
  onOpenFile?: (path: string) => void;
}

function TreeItem({ node, depth, expanded, loaded, onToggle, selectedPath, onOpenFile }: TreeItemProps) {
  const { api } = useApp();
  const isExpanded = expanded.has(node.path);
  const isSelected = node.path === selectedPath;
  const children = loaded.get(node.path);
  const isLoading = isExpanded && children === undefined;

  const handleClick = () => {
    if (node.isDir) onToggle(node);
    else if (onOpenFile) onOpenFile(node.path);
    else api.openCardByPath(node.path);
  };

  return (
    <>
      <div
        className={`tree-row${node.isDir ? " dir" : " file"}${isSelected ? " selected" : ""}`}
        style={{ paddingInlineStart: depth * 14 + 10 }}
        onClick={handleClick}
      >
        {node.isDir ? (
          <span className="tree-chev">
            {isExpanded ? <ChevronDown size={13} strokeWidth={1.8} /> : <ChevronRight size={13} strokeWidth={1.8} />}
          </span>
        ) : (
          <span className="tree-chev-pad" />
        )}
        <span className="tree-name">{node.name}</span>
        {isLoading && <span className="tree-loading">…</span>}
      </div>
      {isExpanded && children?.map((child) => (
        <TreeItem key={child.path} node={child} depth={depth + 1} expanded={expanded} loaded={loaded} onToggle={onToggle} selectedPath={selectedPath} onOpenFile={onOpenFile} />
      ))}
    </>
  );
}

export default function FileTree({ selectedPath: propSelectedPath, onOpenFile }: { selectedPath?: string | null; onOpenFile?: (path: string) => void } = {}) {
  const { state } = useApp();
  const [root, setRoot] = useState<TreeNode[] | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [loaded, setLoaded] = useState<Map<string, TreeNode[]>>(new Map());
  const [error, setError] = useState<string | null>(null);

  const expandedRef = useRef(expanded);
  expandedRef.current = expanded;
  const kbRootRef = useRef(state.kbRoot);
  kbRootRef.current = state.kbRoot;
  const loadedRef = useRef(loaded);
  loadedRef.current = loaded;

  const selectedCard = state.cards.find((c) => c.id === state.selectedId);
  const selectedPath = propSelectedPath !== undefined ? propSelectedPath : (selectedCard?.path ?? null);

  /* 挂载时扫描顶层 */
  useEffect(() => {
    scanDir(state.kbRoot, state.kbRoot)
      .then((nodes) => {
        setRoot(nodes);
        if (nodes.length === 0) setError("知识库为空");
      })
      .catch(() => setError("无法读取 .knowledges/ 目录"));
  }, []);

  /* 选中卡片时自动展开所在目录链路 */
  useEffect(() => {
    if (!selectedPath) return;
    const parts = selectedPath.split("/");
    const dirPaths: string[] = [];
    for (let i = 2; i <= parts.length - 1; i++) dirPaths.push(parts.slice(0, i).join("/"));
    if (dirPaths.length === 0) return;

    setExpanded((prev) => {
      const next = new Set(prev);
      let changed = false;
      for (const p of dirPaths) { if (!next.has(p)) { next.add(p); changed = true; } }
      return changed ? next : prev;
    });

    for (const p of dirPaths) {
      if (!loadedRef.current.has(p)) {
        scanDir(state.kbRoot, p).then((children) => {
          setLoaded((prev) => {
            if (prev.has(p)) return prev;
            const next = new Map(prev);
            next.set(p, children);
            return next;
          });
        });
      }
    }
  }, [selectedPath, state.kbRoot]);

  const handleToggle = useCallback((node: TreeNode) => {
    const willExpand = !expandedRef.current.has(node.path);
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(node.path)) next.delete(node.path);
      else next.add(node.path);
      return next;
    });
    if (willExpand && !loadedRef.current.has(node.path)) {
      scanDir(kbRootRef.current, node.path).then((children) => {
        setLoaded((prev) => {
          if (prev.has(node.path)) return prev;
          const next = new Map(prev);
          next.set(node.path, children);
          return next;
        });
      });
    }
  }, []);

  if (error) return <div className="tree-empty">{error}</div>;
  if (!root) return <div className="tree-empty">…</div>;
  if (root.length === 0) return <div className="tree-empty">知识库为空</div>;

  return (
    <nav className="file-tree">
      {root.map((node) => (
        <TreeItem key={node.path} node={node} depth={0} expanded={expanded} loaded={loaded} onToggle={handleToggle} selectedPath={selectedPath} onOpenFile={onOpenFile} />
      ))}
    </nav>
  );
}
