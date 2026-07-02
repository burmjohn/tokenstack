import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";

const fixtureRoot = path.join(process.cwd(), "src-tauri", "fixtures");
const realLookingPatterns = [
  /"email"\s*:\s*"[^"]+@[^"]+\.[^"]+"/i,
  /"account_id"\s*:\s*"[A-Za-z0-9_-]{12,}"/i,
  /"access_token"\s*:/i,
  /"refresh_token"\s*:/i,
  /sk-[A-Za-z0-9_-]{20,}/,
];

async function exists(dir) {
  return stat(dir).then(() => true, () => false);
}

async function walk(dir, hits = []) {
  if (!(await exists(dir))) {
    return hits;
  }
  for (const entry of await readdir(dir)) {
    const full = path.join(dir, entry);
    const info = await stat(full);
    if (info.isDirectory()) {
      await walk(full, hits);
      continue;
    }
    const text = await readFile(full, "utf8");
    for (const pattern of realLookingPatterns) {
      if (pattern.test(text)) {
        hits.push(path.relative(process.cwd(), full));
      }
    }
  }
  return hits;
}

const hits = await walk(fixtureRoot);
if (hits.length > 0) {
  console.error(`Fixture scan failed:\n${hits.join("\n")}`);
  process.exit(1);
}

console.log("Fixture scan passed: fixtures are synthetic and redacted.");
