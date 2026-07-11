use crate::codex_runtime::{CodexLaunchSpec, CodexRuntimeSource};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredCodexRuntime {
    pub display_path: PathBuf,
    pub launch: CodexLaunchSpec,
    pub source: CodexRuntimeSource,
    pub validated_at_utc: String,
    pub version: String,
}

pub fn load_configured_runtime(
    conn: &Connection,
) -> rusqlite::Result<Option<ConfiguredCodexRuntime>> {
    conn.query_row(
        "SELECT display_path, executable_path, argv_prefix_json, source, validated_at_utc, version FROM codex_runtime_settings WHERE singleton_key = 1",
        [],
        |row| {
            let source_json: String = row.get(3)?;
            let argv_json: String = row.get(2)?;
            let source = serde_json::from_str(&source_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
            })?;
            let argv_prefix = serde_json::from_str(&argv_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(error))
            })?;
            Ok(ConfiguredCodexRuntime {
                display_path: PathBuf::from(row.get::<_, String>(0)?),
                launch: CodexLaunchSpec {
                    executable_path: PathBuf::from(row.get::<_, String>(1)?),
                    argv_prefix,
                },
                source,
                validated_at_utc: row.get(4)?,
                version: row.get(5)?,
            })
        },
    ).optional()
}

pub fn save_configured_runtime(
    conn: &Connection,
    runtime: &ConfiguredCodexRuntime,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO codex_runtime_settings (singleton_key, display_path, executable_path, argv_prefix_json, source, validated_at_utc, version) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(singleton_key) DO UPDATE SET display_path=excluded.display_path, executable_path=excluded.executable_path, argv_prefix_json=excluded.argv_prefix_json, source=excluded.source, validated_at_utc=excluded.validated_at_utc, version=excluded.version",
        params![
            runtime.display_path.to_string_lossy(),
            runtime.launch.executable_path.to_string_lossy(),
            serde_json::to_string(&runtime.launch.argv_prefix).expect("string list serializes"),
            serde_json::to_string(&runtime.source).expect("runtime source serializes"),
            runtime.validated_at_utc,
            runtime.version,
        ],
    )?;
    Ok(())
}

pub fn clear_configured_runtime(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM codex_runtime_settings WHERE singleton_key = 1",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_runtime::{CodexLaunchSpec, CodexRuntimeSource};
    use crate::db::open_path;
    use std::path::PathBuf;

    #[test]
    fn configured_runtime_survives_database_restart_with_prefix_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("settings.sqlite3");
        let selected = ConfiguredCodexRuntime {
            display_path: PathBuf::from("C:/Users/Test/AppData/Roaming/npm/codex.cmd"),
            launch: CodexLaunchSpec {
                executable_path: PathBuf::from("C:/Program Files/nodejs/node.exe"),
                argv_prefix: vec![
                    "C:/Users/Test/AppData/Roaming/npm/node_modules/@openai/codex/bin/codex.js"
                        .into(),
                ],
            },
            source: CodexRuntimeSource::Npm,
            validated_at_utc: "2026-07-10T12:00:00Z".into(),
            version: "codex-cli 1.2.3".into(),
        };

        save_configured_runtime(&open_path(&db).unwrap(), &selected).unwrap();
        let reopened = open_path(&db).unwrap();
        assert_eq!(load_configured_runtime(&reopened).unwrap(), Some(selected));

        clear_configured_runtime(&reopened).unwrap();
        drop(reopened);
        assert_eq!(
            load_configured_runtime(&open_path(&db).unwrap()).unwrap(),
            None
        );
    }
}
