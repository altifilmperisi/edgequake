/**
 * Locale key parity — SPEC-155 LAW-155-9 / EC-155-72.
 * Fails if fr/zh miss keys present in en.json.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const localesDir = path.join(__dirname, "../src/locales");

function flatten(obj, prefix = "", out = {}) {
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object" && !Array.isArray(v)) {
      flatten(v, key, out);
    } else {
      out[key] = v;
    }
  }
  return out;
}

function load(lang) {
  return flatten(
    JSON.parse(fs.readFileSync(path.join(localesDir, `${lang}.json`), "utf8")),
  );
}

const en = load("en");
const fr = load("fr");
const zh = load("zh");

const missingFr = Object.keys(en).filter((k) => !(k in fr));
const missingZh = Object.keys(en).filter((k) => !(k in zh));

let failed = false;
if (missingFr.length) {
  console.error(`fr missing ${missingFr.length} keys (sample):`, missingFr.slice(0, 20));
  failed = true;
}
if (missingZh.length) {
  console.error(`zh missing ${missingZh.length} keys (sample):`, missingZh.slice(0, 20));
  failed = true;
}

if (failed) {
  console.error("SPEC-155 locale_parity FAILED");
  process.exit(1);
}
console.log(
  `SPEC-155 locale_parity OK — en=${Object.keys(en).length} fr=${Object.keys(fr).length} zh=${Object.keys(zh).length}`,
);
