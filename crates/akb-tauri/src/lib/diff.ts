/* 行级 diff(LCS):审核 update 项的双栏对照 */
export interface DiffRow {
  t: "ctx" | "del" | "add";
  l: string;
  r: string;
}

export function diffLines(a: string, b: string): DiffRow[] {
  const A = a.split("\n");
  const B = b.split("\n");
  const m = A.length;
  const n = B.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = m - 1; i >= 0; i--) {
    for (let j = n - 1; j >= 0; j--) {
      dp[i][j] = A[i] === B[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }
  const rows: DiffRow[] = [];
  let i = 0;
  let j = 0;
  while (i < m && j < n) {
    if (A[i] === B[j]) {
      rows.push({ t: "ctx", l: A[i], r: B[j] });
      i++;
      j++;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      rows.push({ t: "del", l: A[i], r: "" });
      i++;
    } else {
      rows.push({ t: "add", l: "", r: B[j] });
      j++;
    }
  }
  while (i < m) rows.push({ t: "del", l: A[i++], r: "" });
  while (j < n) rows.push({ t: "add", l: "", r: B[j++] });
  return rows;
}
