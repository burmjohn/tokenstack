import { LogicalPosition } from "@tauri-apps/api/dpi";
import { Menu, MenuItem, PredefinedMenuItem } from "@tauri-apps/api/menu";
import { isTauriRuntime } from "../../lib/api/tauri";
import type { DesktopMenuCommand } from "./commands";

type ContextMenuHandler = (command: DesktopMenuCommand) => void;

const CONTEXT_ITEMS: Array<
  | { type: "item"; id: string; text: string; command: DesktopMenuCommand }
  | { type: "separator" }
> = [
  { type: "item", id: "desktop.refresh", text: "Refresh data", command: "refresh" },
  { type: "separator" },
  { type: "item", id: "desktop.export-badge", text: "Export badge", command: "export-badge" },
  { type: "item", id: "desktop.export-csv", text: "Export usage CSV", command: "export-csv" },
  { type: "separator" },
  {
    type: "item",
    id: "desktop.navigate-dashboard",
    text: "Dashboard",
    command: "navigate-dashboard",
  },
  { type: "item", id: "desktop.navigate-usage", text: "Usage", command: "navigate-usage" },
  {
    type: "item",
    id: "desktop.navigate-resets",
    text: "Reset credits",
    command: "navigate-resets",
  },
  {
    type: "item",
    id: "desktop.navigate-sources",
    text: "Sources",
    command: "navigate-sources",
  },
  { type: "item", id: "desktop.navigate-setup", text: "Setup", command: "navigate-setup" },
  { type: "separator" },
  {
    type: "item",
    id: "desktop.toggle-theme",
    text: "Toggle theme",
    command: "toggle-theme",
  },
];

export function installDesktopContextMenu(handler: ContextMenuHandler): () => void {
  if (!isTauriRuntime()) {
    return () => {};
  }

  const onContextMenu = (event: MouseEvent) => {
    event.preventDefault();
    void openContextMenu(event, handler);
  };

  document.addEventListener("contextmenu", onContextMenu);
  return () => document.removeEventListener("contextmenu", onContextMenu);
}

async function openContextMenu(event: MouseEvent, handler: ContextMenuHandler) {
  const items = await Promise.all(
    CONTEXT_ITEMS.map((item) => {
      if (item.type === "separator") {
        return PredefinedMenuItem.new({ item: "Separator" });
      }

      return MenuItem.new({
        id: item.id,
        text: item.text,
        action: () => handler(item.command),
      });
    }),
  );

  const menu = await Menu.new({ items });
  await menu.popup(new LogicalPosition(event.clientX, event.clientY));
}
