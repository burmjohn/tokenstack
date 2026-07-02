use crate::analytics::{build_dashboard_summary, DashboardSummaryDto};
use crate::db::open_memory;
use crate::importers::LocalHistoryImporter;
use std::path::PathBuf;

#[tauri::command]
pub fn get_dashboard_summary(data_mode: String) -> Result<DashboardSummaryDto, String> {
    let conn = open_memory().map_err(|error| error.to_string())?;
    build_dashboard_summary(&conn, &data_mode).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn refresh_all(data_mode: String) -> Result<DashboardSummaryDto, String> {
    let conn = open_memory().map_err(|error| error.to_string())?;
    let roots = synthetic_safe_roots();
    let importer = LocalHistoryImporter::new(roots);
    let _summary = importer
        .import_into(&conn)
        .map_err(|error| error.to_string())?;
    build_dashboard_summary(&conn, &data_mode).map_err(|error| error.to_string())
}

fn synthetic_safe_roots() -> Vec<PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    vec![cwd.join("src-tauri").join("fixtures").join("codex-history")]
}
