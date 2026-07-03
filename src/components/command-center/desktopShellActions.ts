import type { DesktopMenuCommand } from "../../features/desktop/commands";
import type { NavSection } from "./sectionModel";

type DesktopShellActions = {
  exportBadge: () => void;
  exportCsv: () => void;
  navigate: (section: NavSection) => void;
  refresh: () => void;
  toggleTheme: () => void;
};

const NAV_COMMANDS: Partial<Record<DesktopMenuCommand, NavSection>> = {
  "navigate-dashboard": "dashboard",
  "navigate-usage": "usage",
  "navigate-resets": "resets",
  "navigate-sources": "sources",
  "navigate-setup": "setup",
};

export function createDesktopShellActionHandler(actions: DesktopShellActions) {
  return (command: DesktopMenuCommand) => {
    const section = NAV_COMMANDS[command];
    if (section) {
      actions.navigate(section);
      return;
    }

    if (command === "refresh") {
      actions.refresh();
      return;
    }

    if (command === "export-badge") {
      actions.exportBadge();
      return;
    }

    if (command === "export-csv") {
      actions.exportCsv();
      return;
    }

    if (command === "toggle-theme") {
      actions.toggleTheme();
    }
  };
}
