import { describe, expect, it, vi } from "vitest";
import { createDesktopShellActionHandler } from "./desktopShellActions";

describe("desktop shell action adapter", () => {
  it("maps native commands to existing shell actions", () => {
    const actions = {
      exportBadge: vi.fn(),
      exportCsv: vi.fn(),
      navigate: vi.fn(),
      refresh: vi.fn(),
      toggleTheme: vi.fn(),
    };
    const handle = createDesktopShellActionHandler(actions);

    handle("navigate-setup");
    handle("refresh");
    handle("export-badge");
    handle("export-csv");
    handle("toggle-theme");

    expect(actions.navigate).toHaveBeenCalledWith("setup");
    expect(actions.refresh).toHaveBeenCalledTimes(1);
    expect(actions.exportBadge).toHaveBeenCalledTimes(1);
    expect(actions.exportCsv).toHaveBeenCalledTimes(1);
    expect(actions.toggleTheme).toHaveBeenCalledTimes(1);
  });
});
