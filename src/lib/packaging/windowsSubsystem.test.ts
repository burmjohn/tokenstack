import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";

describe("Windows packaging", () => {
  it("uses the Windows GUI subsystem for release builds", async () => {
    const main = await readFile("src-tauri/src/main.rs", "utf8");

    expect(main).toContain("windows_subsystem = \"windows\"");
    expect(main).toContain("not(debug_assertions)");
  });
});
