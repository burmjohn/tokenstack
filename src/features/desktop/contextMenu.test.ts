import { Menu, MenuItem, PredefinedMenuItem } from "@tauri-apps/api/menu";
import { afterEach, describe, expect, it, vi } from "vitest";
import { installDesktopContextMenu } from "./contextMenu";

vi.mock("@tauri-apps/api/menu", () => ({
  Menu: { new: vi.fn() },
  MenuItem: { new: vi.fn() },
  PredefinedMenuItem: { new: vi.fn() },
}));

vi.mock("@tauri-apps/api/dpi", () => ({
  LogicalPosition: class LogicalPosition {
    constructor(
      public x: number,
      public y: number,
    ) {}
  },
}));

const menuNew = vi.mocked(Menu.new);
const menuItemNew = vi.mocked(MenuItem.new);
const predefinedNew = vi.mocked(PredefinedMenuItem.new);

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  menuNew.mockReset();
  menuItemNew.mockReset();
  predefinedNew.mockReset();
});

describe("desktop context menu", () => {
  it("does not override browser context menus outside Tauri", () => {
    const handler = vi.fn();
    const remove = installDesktopContextMenu(handler);
    const event = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
    const preventDefault = vi.spyOn(event, "preventDefault");

    document.dispatchEvent(event);
    remove();

    expect(preventDefault).not.toHaveBeenCalled();
    expect(menuNew).not.toHaveBeenCalled();
  });

  it("prevents the WebView default menu and opens the TokenStack menu in Tauri", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    const popup = vi.fn();
    menuNew.mockResolvedValue({ popup } as unknown as Awaited<ReturnType<typeof Menu.new>>);
    menuItemNew.mockImplementation(
      async (opts) => ({ opts }) as unknown as Awaited<ReturnType<typeof MenuItem.new>>,
    );
    predefinedNew.mockImplementation(
      async (opts) => ({ opts }) as unknown as Awaited<ReturnType<typeof PredefinedMenuItem.new>>,
    );
    const handler = vi.fn();

    const remove = installDesktopContextMenu(handler);
    const event = new MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
      clientX: 32,
      clientY: 48,
    });
    const preventDefault = vi.spyOn(event, "preventDefault");
    document.dispatchEvent(event);
    await flushAsyncWork();
    remove();

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(menuNew).toHaveBeenCalled();
    expect(popup).toHaveBeenCalledWith(expect.objectContaining({ x: 32, y: 48 }));
    expect(menuItemNew).toHaveBeenCalledWith(expect.objectContaining({ id: "desktop.refresh" }));
    expect(predefinedNew).toHaveBeenCalledWith({ item: "Separator" });
  });
});

function flushAsyncWork() {
  return new Promise((resolve) => setTimeout(resolve, 0));
}
