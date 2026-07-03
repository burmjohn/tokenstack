import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { downloadTextFile } from "./download";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const originalCreateObjectURL = URL.createObjectURL;
const originalRevokeObjectURL = URL.revokeObjectURL;

describe("downloadTextFile", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    delete (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    restoreUrlApi("createObjectURL", originalCreateObjectURL);
    restoreUrlApi("revokeObjectURL", originalRevokeObjectURL);
  });

  it("saves text exports through Tauri in the installed desktop runtime", async () => {
    (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    vi.mocked(invoke).mockResolvedValue("C:\\Users\\burmj\\Downloads\\tokenstack-diagnostics-2026-07-03.json");

    const result = await downloadTextFile(
      "tokenstack-diagnostics-2026-07-03.json",
      "{\"diagnostics\":true}",
      "application/json;charset=utf-8",
    );

    expect(invoke).toHaveBeenCalledWith("save_text_export", {
      filename: "tokenstack-diagnostics-2026-07-03.json",
      contents: "{\"diagnostics\":true}",
    });
    expect(result).toEqual({
      status: "saved",
      path: "C:\\Users\\burmj\\Downloads\\tokenstack-diagnostics-2026-07-03.json",
    });
  });

  it("keeps browser object URL downloads for non-Tauri development", async () => {
    const createObjectURL = vi.fn<(blob: Blob) => string>(() => "blob:tokenstack-export");
    const revokeObjectURL = vi.fn<(url: string) => void>();
    const anchorClick = vi.fn<() => void>();
    const originalCreateElement = document.createElement.bind(document);
    let anchor: HTMLAnchorElement | undefined;

    Object.defineProperty(URL, "createObjectURL", { configurable: true, value: createObjectURL });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: revokeObjectURL });
    vi.spyOn(document, "createElement").mockImplementation(((tagName: string, options?: ElementCreationOptions) => {
      const element = originalCreateElement(tagName, options);
      if (tagName.toLowerCase() === "a") {
        anchor = element as HTMLAnchorElement;
        vi.spyOn(anchor, "click").mockImplementation(anchorClick);
      }
      return element;
    }) as typeof document.createElement);

    const result = await downloadTextFile("tokenstack-usage-2026-07-03.csv", "metric,value\nlifetime,1", "text/csv");

    expect(invoke).not.toHaveBeenCalled();
    expect(anchorClick).toHaveBeenCalledTimes(1);
    expect(anchor?.download).toBe("tokenstack-usage-2026-07-03.csv");
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:tokenstack-export");
    expect(result).toEqual({ status: "downloaded" });
  });
});

function restoreUrlApi(key: "createObjectURL", value: typeof URL.createObjectURL | undefined): void;
function restoreUrlApi(key: "revokeObjectURL", value: typeof URL.revokeObjectURL | undefined): void;
function restoreUrlApi(key: "createObjectURL" | "revokeObjectURL", value: typeof URL.createObjectURL | typeof URL.revokeObjectURL | undefined) {
  if (value) {
    Object.defineProperty(URL, key, { configurable: true, value });
    return;
  }
  delete (URL as unknown as Record<string, unknown>)[key];
}
