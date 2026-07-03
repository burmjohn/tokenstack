import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { isTauriRuntime } from "../../lib/api/tauri";

export const DESKTOP_MENU_EVENT = "tokenstack://desktop-menu";

export type DesktopMenuCommand =
  | "about"
  | "export-badge"
  | "export-csv"
  | "navigate-dashboard"
  | "navigate-resets"
  | "navigate-setup"
  | "navigate-sources"
  | "navigate-usage"
  | "quit"
  | "refresh"
  | "show-app"
  | "toggle-theme";

type DesktopMenuPayload = {
  command: DesktopMenuCommand;
};

export async function listenForDesktopMenuCommands(
  handler: (command: DesktopMenuCommand) => void,
): Promise<UnlistenFn | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return listen<DesktopMenuPayload>(DESKTOP_MENU_EVENT, (event) => {
    handler(event.payload.command);
  });
}
