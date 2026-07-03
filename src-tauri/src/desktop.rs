use serde::Serialize;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, WebviewWindow};

use crate::desktop_menu::{
    desktop_command_for_id, DesktopCommand, DesktopMenuEntry, DesktopMenuKind, APP_MENU_SECTIONS,
    MENU_EVENT_NAME, TRAY_MENU_ENTRIES,
};

#[derive(Serialize)]
struct DesktopMenuPayload {
    command: DesktopCommand,
}

pub fn install(app: &mut App) -> tauri::Result<()> {
    let menu = build_app_menu(app)?;
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| handle_menu_id(app, event.id().as_ref()));
    install_tray(app)?;
    Ok(())
}

fn build_app_menu(app: &App) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    let mut menu = MenuBuilder::new(app);

    for section in APP_MENU_SECTIONS {
        let mut submenu = SubmenuBuilder::new(app, section.label);
        for entry in section.entries {
            match entry.kind {
                DesktopMenuKind::Item => {
                    let id = entry.id.expect("desktop menu item id");
                    let mut item = MenuItemBuilder::with_id(id, entry.label);
                    if let Some(accelerator) = entry.accelerator {
                        item = item.accelerator(accelerator);
                    }
                    submenu = submenu.item(&item.build(app)?);
                }
                DesktopMenuKind::Separator => {
                    submenu = submenu.separator();
                }
                DesktopMenuKind::NativeAbout => {
                    submenu =
                        submenu.item(&PredefinedMenuItem::about(app, Some(entry.label), None)?);
                }
                DesktopMenuKind::NativeQuit => {
                    submenu = submenu.item(&PredefinedMenuItem::quit(app, Some(entry.label))?);
                }
                DesktopMenuKind::NativeCloseWindow => {
                    submenu =
                        submenu.item(&PredefinedMenuItem::close_window(app, Some(entry.label))?);
                }
                DesktopMenuKind::NativeMinimize => {
                    submenu = submenu.item(&PredefinedMenuItem::minimize(app, Some(entry.label))?);
                }
            }
        }
        menu = menu.item(&submenu.build()?);
    }

    menu.build()
}

fn install_tray(app: &mut App) -> tauri::Result<()> {
    let tray_menu = build_tray_menu(app)?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("TokenStack")
        .show_menu_on_left_click(false)
        .menu(&tray_menu)
        .on_menu_event(|app, event| handle_menu_id(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                focus_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

fn build_tray_menu(app: &App) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    let mut menu = MenuBuilder::new(app);
    for entry in TRAY_MENU_ENTRIES {
        menu = append_tray_menu_entry(menu, app, entry)?;
    }
    menu.build()
}

fn append_tray_menu_entry<'a>(
    menu: MenuBuilder<'a, tauri::Wry, App>,
    app: &'a App,
    entry: &DesktopMenuEntry,
) -> tauri::Result<MenuBuilder<'a, tauri::Wry, App>> {
    match entry.kind {
        DesktopMenuKind::Item => {
            let id = entry.id.expect("tray menu item id");
            Ok(menu.item(&MenuItemBuilder::with_id(id, entry.label).build(app)?))
        }
        DesktopMenuKind::Separator => Ok(menu.separator()),
        DesktopMenuKind::NativeQuit => {
            Ok(menu.item(&PredefinedMenuItem::quit(app, Some(entry.label))?))
        }
        DesktopMenuKind::NativeAbout
        | DesktopMenuKind::NativeCloseWindow
        | DesktopMenuKind::NativeMinimize => Ok(menu),
    }
}

fn handle_menu_id(app: &AppHandle, id: &str) {
    match desktop_command_for_id(id) {
        Some(DesktopCommand::ShowApp) => focus_main_window(app),
        Some(DesktopCommand::Quit) => app.exit(0),
        Some(command) => {
            let _ = app.emit(MENU_EVENT_NAME, DesktopMenuPayload { command });
        }
        None => {}
    }
}

fn focus_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        show_and_focus(&window);
    }
}

fn show_and_focus(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}
