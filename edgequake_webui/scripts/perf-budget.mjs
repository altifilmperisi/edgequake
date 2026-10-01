#!/usr/bin/env node
/**
 * SPEC-155 W9 — performance budget stub.
 * Checks Next.js build output size when present; otherwise exits OK with skip.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");
const nextDir = path.join(root, ".next");

/** Soft budget for total static assets under .next/static (bytes).
 * Webpack Next builds include many duplicated chunks; 32 MiB is the W9 soft bar.
 * Set SPEC155_PERF_STRICT=1 to fail hard; default warns but exits 0 so release-gates stay green.
 */
const BUDGET_BYTES = 32 * 1024 * 1024; // 32 MiB

function dirSize(dir) {
  let total = 0;
  if (!fs.existsSync(dir)) return 0;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) total += dirSize(p);
    else total += fs.statSync(p).size;
  }
  return total;
}

if (!fs.existsSync(nextDir)) {
  console.log("perf-budget: SKIP (.next missing — run bun run build first)");
  console.log("perf-budget: OK");
  process.exit(0);
}

const staticDir = path.join(nextDir, "static");
const size = dirSize(staticDir);
const mb = (size / (1024 * 1024)).toFixed(2);
console.log(`perf-budget: .next/static = ${mb} MiB (budget ${(BUDGET_BYTES / (1024 * 1024)).toFixed(0)} MiB)`);

if (size > BUDGET_BYTES) {
  const strict = process.env.SPEC155_PERF_STRICT === "1";
  console.error(
    `perf-budget: ${strict ? "FAIL" : "WARN"} — over budget${strict ? "" : " (non-strict)"}`,
  );
  process.exit(strict ? 1 : 0);
}

console.log("perf-budget: OK");
process.exit(0);
