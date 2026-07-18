#![cfg_attr(not(feature = "tauri-app"), allow(dead_code))]

mod analytics;
mod codex_app_server;
pub mod codex_runtime;
#[cfg(feature = "tauri-app")]
mod commands;
mod db;
#[cfg(feature = "tauri-app")]
mod desktop;
mod desktop_menu;
mod discovery;
mod importers;
mod settings;
mod telemetry;

use std::path::PathBuf;

const PACKAGED_SMOKE_FLAG: &str = "--tokenstack-packaged-smoke";
const PACKAGED_SMOKE_OPT_IN: &str = "TOKENSTACK_ENABLE_PACKAGED_SMOKE";

#[derive(Debug, PartialEq, Eq)]
pub struct PackagedSmokeRequest {
    pub runtime: Option<PathBuf>,
    pub app_data_dir: PathBuf,
    pub diagnostics_dir: PathBuf,
}

pub fn packaged_smoke_request(
    args: &[String],
    opt_in: Option<&str>,
) -> Option<Result<PackagedSmokeRequest, String>> {
    if !args.iter().any(|arg| arg == PACKAGED_SMOKE_FLAG) {
        return None;
    }
    if opt_in != Some("1") {
        return None;
    }
    let mut runtime = None;
    let mut app_data_dir = None;
    let mut diagnostics_dir = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            PACKAGED_SMOKE_FLAG => index += 1,
            "--runtime" | "--app-data-dir" | "--diagnostics-dir" => {
                let flag = args[index].as_str();
                let Some(value) = args.get(index + 1) else {
                    return Some(Err(format!("missing value for {flag}")));
                };
                let path = PathBuf::from(value);
                match flag {
                    "--runtime" => runtime = Some(path),
                    "--app-data-dir" => app_data_dir = Some(path),
                    _ => diagnostics_dir = Some(path),
                }
                index += 2;
            }
            argument => {
                return Some(Err(format!(
                    "unsupported packaged smoke argument: {argument}"
                )));
            }
        }
    }
    Some(match (app_data_dir, diagnostics_dir) {
        (Some(app_data_dir), Some(diagnostics_dir)) => Ok(PackagedSmokeRequest {
            runtime,
            app_data_dir,
            diagnostics_dir,
        }),
        _ => Err("packaged smoke requires --app-data-dir and --diagnostics-dir".into()),
    })
}

#[cfg(feature = "tauri-app")]
pub fn run_packaged_smoke(request: PackagedSmokeRequest) -> Result<PathBuf, String> {
    commands::run_packaged_smoke(
        request.runtime,
        request.app_data_dir,
        request.diagnostics_dir,
    )
}

pub fn packaged_smoke_from_process() -> Option<Result<PathBuf, String>> {
    let args = std::env::args().collect::<Vec<_>>();
    let opt_in = std::env::var(PACKAGED_SMOKE_OPT_IN).ok();
    packaged_smoke_request(&args, opt_in.as_deref()).map(|request| {
        #[cfg(feature = "tauri-app")]
        {
            run_packaged_smoke(request?)
        }
        #[cfg(not(feature = "tauri-app"))]
        {
            let _ = request?;
            Err("packaged smoke requires the tauri-app feature".into())
        }
    })
}

#[cfg(feature = "tauri-app")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_window_state::Builder::default().build())?;
            desktop::install(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_summary,
            commands::get_setup_diagnostics,
            commands::export_diagnostics,
            commands::refresh_all,
            commands::save_text_export,
            commands::save_binary_export,
            commands::list_codex_runtimes,
            commands::select_codex_runtime,
            commands::choose_codex_runtime,
            commands::clear_codex_runtime,
            commands::validate_codex_runtime
        ])
        .run(tauri::generate_context!())
        .expect("error while running TokenStack");
}

#[cfg(test)]
mod packaged_smoke_tests {
    use super::*;

    #[test]
    fn packaged_smoke_requires_flag_opt_in_and_exact_arguments() {
        let args = vec![
            "tokenstack.exe".to_string(),
            "--tokenstack-packaged-smoke".to_string(),
            "--runtime".to_string(),
            r"C:\runtime with spaces\codex.exe".to_string(),
            "--app-data-dir".to_string(),
            r"C:\smoke data".to_string(),
            "--diagnostics-dir".to_string(),
            r"C:\smoke evidence".to_string(),
        ];

        assert!(packaged_smoke_request(&args, None).is_none());
        let request = packaged_smoke_request(&args, Some("1")).unwrap().unwrap();
        assert_eq!(
            request.runtime,
            Some(std::path::PathBuf::from(
                r"C:\runtime with spaces\codex.exe"
            ))
        );
        assert_eq!(
            request.diagnostics_dir,
            std::path::PathBuf::from(r"C:\smoke evidence")
        );
    }

    #[test]
    fn packaged_smoke_rejects_unknown_or_missing_arguments() {
        let automatic = vec![
            "tokenstack.exe".to_string(),
            "--tokenstack-packaged-smoke".to_string(),
            "--app-data-dir".to_string(),
            r"C:\automatic smoke".to_string(),
            "--diagnostics-dir".to_string(),
            r"C:\automatic smoke\diagnostics".to_string(),
        ];
        let request = packaged_smoke_request(&automatic, Some("1"))
            .unwrap()
            .unwrap();
        assert_eq!(request.runtime, None);

        let unknown = vec![
            "tokenstack.exe".to_string(),
            "--tokenstack-packaged-smoke".to_string(),
            "--app-data-dir".to_string(),
            r"C:\automatic smoke".to_string(),
            "--diagnostics-dir".to_string(),
            r"C:\automatic smoke\diagnostics".to_string(),
            "--unsafe-extra".to_string(),
        ];
        assert!(packaged_smoke_request(&unknown, Some("1"))
            .unwrap()
            .is_err());

        let normal_launch = vec!["tokenstack.exe".to_string()];
        assert!(packaged_smoke_request(&normal_launch, Some("1")).is_none());
    }
}
