/** SPEC-151 — parse/format page range strings (`1-3,7`). */

export function parsePageRange(input: string, pageCount?: number): number[] {
  const s = input.trim();
  if (!s) return [];
  const set = new Set<number>();
  for (const part of s.split(",")) {
    const p = part.trim();
    if (!p) continue;
    if (p.includes("-")) {
      const [a, b] = p.split("-").map((x) => parseInt(x.trim(), 10));
      if (!Number.isFinite(a) || !Number.isFinite(b) || a < 1 || a > b) {
        throw new Error(`Invalid range: ${p}`);
      }
      for (let i = a; i <= b; i++) {
        if (pageCount !== undefined && i > pageCount) {
          throw new Error(`Page ${i} out of range (max ${pageCount})`);
        }
        set.add(i);
      }
    } else {
      const n = parseInt(p, 10);
      if (!Number.isFinite(n) || n < 1) throw new Error(`Invalid page: ${p}`);
      if (pageCount !== undefined && n > pageCount) {
        throw new Error(`Page ${n} out of range (max ${pageCount})`);
      }
      set.add(n);
    }
  }
  return Array.from(set).sort((a, b) => a - b);
}

export function formatPageRange(pages: number[]): string {
  if (pages.length === 0) return "";
  const sorted = [...pages].sort((a, b) => a - b);
  const parts: string[] = [];
  let start = sorted[0];
  let prev = sorted[0];
  for (let i = 1; i <= sorted.length; i++) {
    const cur = sorted[i];
    if (cur === prev + 1) {
      prev = cur;
      continue;
    }
    parts.push(start === prev ? String(start) : `${start}-${prev}`);
    start = cur;
    prev = cur;
  }
  return parts.join(",");
}

export function togglePage(
  selected: number[],
  page: number,
  shiftAnchor: number | null,
): { pages: number[]; anchor: number } {
  if (shiftAnchor !== null && shiftAnchor !== page) {
    const [lo, hi] = page < shiftAnchor ? [page, shiftAnchor] : [shiftAnchor, page];
    const set = new Set(selected);
    for (let i = lo; i <= hi; i++) set.add(i);
    return { pages: Array.from(set).sort((a, b) => a - b), anchor: page };
  }
  const set = new Set(selected);
  if (set.has(page)) set.delete(page);
  else set.add(page);
  return { pages: Array.from(set).sort((a, b) => a - b), anchor: page };
}
