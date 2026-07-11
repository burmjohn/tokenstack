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

  it("runs the Windows connector smoke only through an installed packaged executable", async () => {
    const script = await readFile("scripts/windows-smoke.ps1", "utf8");
    const workflow = await readFile(".github/workflows/ci.yml", "utf8");

    expect(script).toContain("TOKENSTACK_ENABLE_PACKAGED_SMOKE");
    expect(script).toContain("--tokenstack-packaged-smoke");
    expect(script).toContain("fake runtime with spaces");
    expect(script).toContain('Invoke-PackagedSmoke -Mode "explicit"');
    expect(script).toContain('Invoke-PackagedSmoke -Mode "automatic"');
    expect(script).toContain("Remove-Item Env:TOKENSTACK_CODEX_BIN");
    expect(script).toContain('$env:PATH = @(');
    expect(script).toContain("Uninstall\\*");
    expect(script).toContain("refuses a build-tree executable");
    expect(script).not.toContain('target\\release\\tokenstack.exe")');
    expect(workflow).toContain("-InstallerPath $installer.FullName");
    expect(workflow).toContain("tokenstack-windows-packaged-smoke-diagnostics");
    expect(workflow).toContain("if: always()");
  });
});
