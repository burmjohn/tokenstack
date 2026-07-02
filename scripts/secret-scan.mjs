import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";

const root = process.cwd();
const ignored = new Set(["node_modules", ".git", "dist", "target", "playwright-report", "test-results"]);
const ignoredFiles = new Set(["scripts/secret-scan.mjs", "scripts/fixture-scan.mjs"]);
const secretPatterns = [
  /sk-[A-Za-z0-9_-]{20,}/,
  /xox[baprs]-[A-Za-z0-9-]{20,}/,
  /gh[pousr]_[A-Za-z0-9_]{20,}/,
  /Bearer\s+[A-Za-z0-9._-]{20,}/i,
  /"access_token"\s*:\s*"[A-Za-z0-9._-]{20,}"/i,
  /"refresh_token"\s*:\s*"[A-Za-z0-9._-]{20,}"/i,
];

const hits = [];

async function walk(dir) {
  for (const entry of await readdir(dir)) {
    if (ignored.has(entry) || entry.endsWith(".png") || entry.endsWith(".jpg") || entry.endsWith(".jpeg")) {
      continue;
    }
    const full = path.join(dir, entry);
    const relative = path.relative(root, full);
    if (ignoredFiles.has(relative)) {
      continue;
    }
    const info = await stat(full);
    if (info.isDirectory()) {
      await walk(full);
      continue;
    }
    if (info.size > 1_000_000) {
      continue;
    }
    const text = await readFile(full, "utf8").catch(() => "");
    for (const pattern of secretPatterns) {
      if (pattern.test(text)) {
        hits.push(path.relative(root, full));
      }
    }
  }
}

await walk(root);

if (hits.length > 0) {
  console.error(`Secret scan failed:\n${hits.join("\n")}`);
  process.exit(1);
}

console.log("Secret scan passed: no auth-like token patterns found.");
