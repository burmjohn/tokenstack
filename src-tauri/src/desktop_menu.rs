use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DesktopCommand {
    ExportBadge,
    ExportCsv,
    NavigateDashboard,
    NavigateResets,
    NavigateSetup,
    NavigateSources,
    NavigateUsage,
    Refresh,
    ShowApp,
    ToggleTheme,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopMenuKind {
    Item,
    Separator,
    NativeAbout,
    NativeQuit,
    NativeCloseWindow,
    NativeMinimize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DesktopMenuEntry {
    pub id: Option<&'static str>,
    pub label: &'static str,
    pub accelerator: Option<&'static str>,
    pub command: Option<DesktopCommand>,
    pub kind: DesktopMenuKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DesktopMenuSection {
    pub label: &'static str,
    pub entries: &'static [DesktopMenuEntry],
}

pub const MENU_EVENT_NAME: &str = "tokenstack://desktop-menu";

const APP_SECTION_ENTRIES: &[DesktopMenuEntry] = &[
    native(
        "desktop.about",
        "About TokenStack",
        DesktopMenuKind::NativeAbout,
    ),
    separator(),
    item(
        "desktop.show-app",
        "Show TokenStack",
        Some("CmdOrCtrl+0"),
        DesktopCommand::ShowApp,
    ),
    separator(),
    native(
        "desktop.quit",
        "Quit TokenStack",
        DesktopMenuKind::NativeQuit,
    ),
];

const FILE_SECTION_ENTRIES: &[DesktopMenuEntry] = &[
    item(
        "desktop.refresh",
        "Refresh data",
        Some("CmdOrCtrl+R"),
        DesktopCommand::Refresh,
    ),
    separator(),
    item(
        "desktop.export-badge",
        "Export badge",
        Some("CmdOrCtrl+B"),
        DesktopCommand::ExportBadge,
    ),
    item(
        "desktop.export-csv",
        "Export usage CSV",
        Some("CmdOrCtrl+E"),
        DesktopCommand::ExportCsv,
    ),
];

const VIEW_SECTION_ENTRIES: &[DesktopMenuEntry] = &[
    item(
        "desktop.navigate-dashboard",
        "Dashboard",
        Some("CmdOrCtrl+1"),
        DesktopCommand::NavigateDashboard,
    ),
    item(
        "desktop.navigate-usage",
        "Usage",
        Some("CmdOrCtrl+2"),
        DesktopCommand::NavigateUsage,
    ),
    item(
        "desktop.navigate-resets",
        "Reset credits",
        Some("CmdOrCtrl+3"),
        DesktopCommand::NavigateResets,
    ),
    item(
        "desktop.navigate-sources",
        "Sources",
        Some("CmdOrCtrl+4"),
        DesktopCommand::NavigateSources,
    ),
    item(
        "desktop.navigate-setup",
        "Setup",
        Some("CmdOrCtrl+,"),
        DesktopCommand::NavigateSetup,
    ),
    separator(),
    item(
        "desktop.toggle-theme",
        "Toggle theme",
        Some("CmdOrCtrl+Shift+T"),
        DesktopCommand::ToggleTheme,
    ),
];

const WINDOW_SECTION_ENTRIES: &[DesktopMenuEntry] = &[
    native(
        "desktop.minimize",
        "Minimize",
        DesktopMenuKind::NativeMinimize,
    ),
    native(
        "desktop.close-window",
        "Close window",
        DesktopMenuKind::NativeCloseWindow,
    ),
];

pub const APP_MENU_SECTIONS: &[DesktopMenuSection] = &[
    DesktopMenuSection {
        label: "TokenStack",
        entries: APP_SECTION_ENTRIES,
    },
    DesktopMenuSection {
        label: "File",
        entries: FILE_SECTION_ENTRIES,
    },
    DesktopMenuSection {
        label: "View",
        entries: VIEW_SECTION_ENTRIES,
    },
    DesktopMenuSection {
        label: "Window",
        entries: WINDOW_SECTION_ENTRIES,
    },
];

pub const CONTEXT_MENU_ENTRIES: &[DesktopMenuEntry] = &[
    item(
        "desktop.refresh",
        "Refresh data",
        None,
        DesktopCommand::Refresh,
    ),
    separator(),
    item(
        "desktop.export-badge",
        "Export badge",
        None,
        DesktopCommand::ExportBadge,
    ),
    item(
        "desktop.export-csv",
        "Export usage CSV",
        None,
        DesktopCommand::ExportCsv,
    ),
    separator(),
    item(
        "desktop.navigate-dashboard",
        "Dashboard",
        None,
        DesktopCommand::NavigateDashboard,
    ),
    item(
        "desktop.navigate-usage",
        "Usage",
        None,
        DesktopCommand::NavigateUsage,
    ),
    item(
        "desktop.navigate-resets",
        "Reset credits",
        None,
        DesktopCommand::NavigateResets,
    ),
    item(
        "desktop.navigate-sources",
        "Sources",
        None,
        DesktopCommand::NavigateSources,
    ),
    item(
        "desktop.navigate-setup",
        "Setup",
        None,
        DesktopCommand::NavigateSetup,
    ),
    separator(),
    item(
        "desktop.toggle-theme",
        "Toggle theme",
        None,
        DesktopCommand::ToggleTheme,
    ),
];

pub const TRAY_MENU_ENTRIES: &[DesktopMenuEntry] = &[
    item(
        "desktop.show-app",
        "Show TokenStack",
        None,
        DesktopCommand::ShowApp,
    ),
    item(
        "desktop.refresh",
        "Refresh data",
        None,
        DesktopCommand::Refresh,
    ),
    separator(),
    item(
        "desktop.export-csv",
        "Export usage CSV",
        None,
        DesktopCommand::ExportCsv,
    ),
    separator(),
    native(
        "desktop.quit",
        "Quit TokenStack",
        DesktopMenuKind::NativeQuit,
    ),
];

pub fn desktop_command_for_id(id: &str) -> Option<DesktopCommand> {
    APP_MENU_SECTIONS
        .iter()
        .flat_map(|section| section.entries.iter())
        .chain(CONTEXT_MENU_ENTRIES.iter())
        .chain(TRAY_MENU_ENTRIES.iter())
        .find(|entry| entry.id == Some(id))
        .and_then(|entry| entry.command)
}

#[cfg(test)]
fn all_user_visible_desktop_copy() -> Vec<&'static str> {
    APP_MENU_SECTIONS
        .iter()
        .flat_map(|section| {
            std::iter::once(section.label).chain(section.entries.iter().map(|entry| entry.label))
        })
        .chain(CONTEXT_MENU_ENTRIES.iter().map(|entry| entry.label))
        .chain(TRAY_MENU_ENTRIES.iter().map(|entry| entry.label))
        .collect()
}

const fn item(
    id: &'static str,
    label: &'static str,
    accelerator: Option<&'static str>,
    command: DesktopCommand,
) -> DesktopMenuEntry {
    DesktopMenuEntry {
        id: Some(id),
        label,
        accelerator,
        command: Some(command),
        kind: DesktopMenuKind::Item,
    }
}

const fn native(id: &'static str, label: &'static str, kind: DesktopMenuKind) -> DesktopMenuEntry {
    DesktopMenuEntry {
        id: Some(id),
        label,
        accelerator: None,
        command: None,
        kind,
    }
}

const fn separator() -> DesktopMenuEntry {
    DesktopMenuEntry {
        id: None,
        label: "",
        accelerator: None,
        command: None,
        kind: DesktopMenuKind::Separator,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_menu_contains_expected_desktop_commands() {
        let ids: Vec<&str> = APP_MENU_SECTIONS
            .iter()
            .flat_map(|section| section.entries.iter())
            .filter_map(|entry| entry.id)
            .collect();

        assert!(ids.contains(&"desktop.refresh"));
        assert!(ids.contains(&"desktop.export-badge"));
        assert!(ids.contains(&"desktop.export-csv"));
        assert!(ids.contains(&"desktop.navigate-dashboard"));
        assert!(ids.contains(&"desktop.navigate-usage"));
        assert!(ids.contains(&"desktop.navigate-resets"));
        assert!(ids.contains(&"desktop.navigate-sources"));
        assert!(ids.contains(&"desktop.navigate-setup"));
        assert!(ids.contains(&"desktop.toggle-theme"));
        assert!(ids.contains(&"desktop.show-app"));
        assert!(ids.contains(&"desktop.quit"));
    }

    #[test]
    fn menu_ids_map_to_frontend_commands() {
        assert_eq!(
            desktop_command_for_id("desktop.navigate-setup"),
            Some(DesktopCommand::NavigateSetup)
        );
        assert_eq!(
            desktop_command_for_id("desktop.export-csv"),
            Some(DesktopCommand::ExportCsv)
        );
        assert_eq!(
            desktop_command_for_id("desktop.show-app"),
            Some(DesktopCommand::ShowApp)
        );
        assert_eq!(desktop_command_for_id("missing"), None);
    }

    #[test]
    fn user_visible_desktop_copy_omits_internal_safety_language() {
        let copy = all_user_visible_desktop_copy().join(" ");

        for forbidden in [
            "Read-only",
            "read-only",
            "/consume",
            "Never /consume",
            "Undocumented (RO)",
            "schema-gated",
        ] {
            assert!(
                !copy.contains(forbidden),
                "desktop copy leaked forbidden term: {forbidden}"
            );
        }
    }

    #[test]
    fn context_and_tray_menus_share_command_ids_with_app_menu() {
        let app_ids: Vec<&str> = APP_MENU_SECTIONS
            .iter()
            .flat_map(|section| section.entries.iter())
            .filter_map(|entry| entry.id)
            .collect();

        for entry in CONTEXT_MENU_ENTRIES.iter().chain(TRAY_MENU_ENTRIES.iter()) {
            if let Some(id) = entry.id {
                assert!(
                    app_ids.contains(&id),
                    "context or tray command {id} must also exist in app menu"
                );
            }
        }
    }
}
