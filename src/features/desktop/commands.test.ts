import { listen } from "@tauri-apps/api/event";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  DESKTOP_MENU_EVENT,
  listenForDesktopMenuCommands,
  type DesktopMenuCommand,
} from "./commands";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

const listenMock = vi.mocked(listen);

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  listenMock.mockReset();
});

describe("desktop command bridge", () => {
  it("does not subscribe outside Tauri", async () => {
    const handler = vi.fn();

    await expect(listenForDesktopMenuCommands(handler)).resolves.toBeNull();

    expect(listenMock).not.toHaveBeenCalled();
  });

  it("subscribes to the Tauri desktop menu event and forwards typed commands", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    const unlisten = vi.fn();
    let callback:
      | ((event: { payload: { command: DesktopMenuCommand } }) => void)
      | undefined;
    listenMock.mockImplementation(async (_event, cb) => {
      callback = cb as typeof callback;
      return unlisten;
    });
    const handler = vi.fn();

    await expect(listenForDesktopMenuCommands(handler)).resolves.toBe(unlisten);
    callback?.({ payload: { command: "navigate-setup" } });

    expect(listenMock).toHaveBeenCalledWith(DESKTOP_MENU_EVENT, expect.any(Function));
    expect(handler).toHaveBeenCalledWith("navigate-setup");
  });
});
