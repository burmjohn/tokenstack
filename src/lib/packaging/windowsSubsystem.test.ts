import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";

describe("Windows packaging", () => {
  it("uses the Windows GUI subsystem for release builds", async () => {
    const main = await readFile("src-tauri/src/main.rs", "utf8");

    expect(main).toContain("windows_subsystem = \"windows\"");
    expect(main).toContain("not(debug_assertions)");
  });

  it("registers native desktop shell setup without adding shell execution", async () => {
    const lib = await readFile("src-tauri/src/lib.rs", "utf8");
    const cargo = await readFile("src-tauri/Cargo.toml", "utf8");

    expect(lib).toContain("mod desktop;");
    expect(lib).toContain("desktop::install(app)?");
    expect(lib).not.toContain("tauri_plugin_shell");
    expect(cargo).not.toContain("tauri-plugin-shell");
  });
});
