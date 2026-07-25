/* fs.ts — 目录扫描封装：通过 Tauri invoke 调用 Rust scan_dir 命令 */
import { invoke } from "@tauri-apps/api/core";

export interface TreeNode {
  name: string;
  path: string;
  isDir: boolean;
  children?: TreeNode[];
}

interface DirEntry {
  name: string;
  path: string;
  is_dir: boolean;
}

export async function scanDir(kbRoot: string, dirPath: string): Promise<TreeNode[]> {
  let entries: DirEntry[];
  try {
    entries = await invoke<DirEntry[]>("scan_dir", { kbRoot, dirPath });
  } catch {
    return [];
  }

  return entries.map((e) => ({
    name: e.name,
    path: e.path,
    isDir: e.is_dir,
  }));
}
