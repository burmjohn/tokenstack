import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";

describe("Tauri desktop packaging config", () => {
  it("uses a stable main window label and desktop-sized responsive bounds", async () => {
    const config = JSON.parse(await readFile("src-tauri/tauri.conf.json", "utf8"));
    const [mainWindow] = config.app.windows;

    expect(mainWindow.label).toBe("main");
    expect(mainWindow.title).toBe("TokenStack");
    expect(mainWindow.resizable).toBe(true);
    expect(mainWindow.width).toBeGreaterThanOrEqual(1280);
    expect(mainWindow.height).toBeGreaterThanOrEqual(820);
    expect(mainWindow.minWidth).toBeLessThanOrEqual(1040);
    expect(mainWindow.minHeight).toBeLessThanOrEqual(680);
  });

  it("keeps bundle descriptions user-facing and free of internal safety copy", async () => {
    const configText = await readFile("src-tauri/tauri.conf.json", "utf8");
    const config = JSON.parse(configText);
    const bundleCopy = [
      config.bundle.shortDescription,
      config.bundle.longDescription,
    ].join(" ");

    expect(bundleCopy).toContain("Local Codex usage");
    expect(bundleCopy).not.toMatch(/read-only/i);
    expect(bundleCopy).not.toContain("/consume");
    expect(bundleCopy).not.toMatch(/undocumented|schema-gated/i);
    expect(configText).not.toContain("Never /consume");
  });

  it("registers the window-state plugin without shell or filesystem plugins", async () => {
    const cargo = await readFile("src-tauri/Cargo.toml", "utf8");
    const lib = await readFile("src-tauri/src/lib.rs", "utf8");

    expect(cargo).toContain("tauri-plugin-window-state");
    expect(lib).toContain("tauri_plugin_window_state::Builder::default().build()");
    expect(cargo).not.toContain("tauri-plugin-shell");
    expect(cargo).not.toContain("tauri-plugin-fs");
  });

  it("keeps package metadata free of internal safety copy", async () => {
    const packageJson = JSON.parse(await readFile("package.json", "utf8"));

    expect(packageJson.description).toContain("Local Codex usage");
    expect(packageJson.description).not.toMatch(/read-only/i);
    expect(packageJson.description).not.toContain("/consume");
  });
});
