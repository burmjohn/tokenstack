use crate::db::{
    insert_import_run, insert_usage_event, record_source_coverage, upsert_source_document,
    ImportRunSummary, UsageEvent,
};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LocalHistoryImporter {
    roots: Vec<PathBuf>,
}

impl LocalHistoryImporter {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }

    pub fn import_into(&self, conn: &Connection) -> anyhow::Result<ImportRunSummary> {
        let mut summary = ImportRunSummary {
            files_seen: 0,
            events_seen: 0,
            events_imported: 0,
            warnings: Vec::new(),
        };

        for file in self.discover_files()? {
            summary.files_seen += 1;
            let text = fs::read_to_string(&file)?;
            let path_hash = hash_text(&file.to_string_lossy());
            let content_hash = hash_text(&text);
            let safe_label = file
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("history.jsonl");
            let doc_id = upsert_source_document(
                conn,
                "local-codex-history",
                &path_hash,
                safe_label,
                &content_hash,
                text.len() as i64,
            )?;

            for (line_index, line) in text.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                summary.events_seen += 1;
                match parse_usage_event(line, doc_id, &path_hash, line_index) {
                    Ok(Some(event)) => {
                        if insert_usage_event(conn, &event)? {
                            summary.events_imported += 1;
                        }
                    }
                    Ok(None) => summary.warnings.push(format!(
                        "{safe_label}:{} unknown event shape skipped",
                        line_index + 1
                    )),
                    Err(error) => {
                        summary
                            .warnings
                            .push(format!("{safe_label}:{} {}", line_index + 1, error))
                    }
                }
            }
        }

        let (coverage, confidence, explanation, missing) = local_coverage(&summary);
        record_source_coverage(
            conn,
            "local-usage",
            "Local history",
            coverage,
            confidence,
            &missing,
            explanation,
        )?;
        insert_import_run(conn, &summary)?;
        Ok(summary)
    }

    fn discover_files(&self) -> anyhow::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for root in &self.roots {
            collect_jsonl(root, &mut files)?;
        }
        files.sort();
        Ok(files)
    }
}

fn collect_jsonl(path: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        collect_jsonl(&entry?.path(), files)?;
    }
    Ok(())
}

pub fn parse_usage_event(
    line: &str,
    doc_id: i64,
    path_hash: &str,
    line_index: usize,
) -> anyhow::Result<Option<UsageEvent>> {
    let value: Value = serde_json::from_str(line)?;
    let outer_event_kind = value
        .get("type")
        .or_else(|| value.get("event"))
        .or_else(|| value.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let event_kind = value
        .pointer("/payload/type")
        .and_then(Value::as_str)
        .unwrap_or(outer_event_kind);

    let usage = value
        .get("usage")
        .or_else(|| value.pointer("/payload/usage"))
        .or_else(|| value.pointer("/payload/info/last_token_usage"))
        .or_else(|| value.pointer("/payload/info/total_token_usage"))
        .unwrap_or(&value);
    let input = number_at(usage, &["input_tokens", "input", "prompt_tokens"]);
    let output = number_at(usage, &["output_tokens", "output", "completion_tokens"]);
    let cache_read = number_at(usage, &["cache_read_tokens", "cached_input_tokens"]);
    let cache_write = number_at(usage, &["cache_write_tokens"]);
    let explicit_total = number_at(usage, &["total_tokens", "tokens"]);
    let total = explicit_total.unwrap_or_else(|| {
        input.unwrap_or(0)
            + output.unwrap_or(0)
            + cache_read.unwrap_or(0)
            + cache_write.unwrap_or(0)
    });

    if total <= 0 {
        return Ok(None);
    }

    let occurred_at_utc = value
        .get("timestamp")
        .or_else(|| value.get("created_at"))
        .or_else(|| value.get("time"))
        .and_then(Value::as_str)
        .and_then(parse_utc)
        .ok_or_else(|| anyhow::anyhow!("missing or invalid timestamp"))?;

    let session_uid = value
        .get("session_id")
        .or_else(|| value.get("sessionId"))
        .or_else(|| value.get("conversation_id"))
        .or_else(|| value.pointer("/payload/session_id"))
        .or_else(|| value.pointer("/payload/sessionId"))
        .or_else(|| value.pointer("/payload/conversation_id"))
        .and_then(Value::as_str)
        .unwrap_or(path_hash)
        .to_string();

    let source_uid = value
        .get("id")
        .or_else(|| value.get("event_id"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| hash_text(&format!("{path_hash}:{line_index}:{line}")));

    Ok(Some(UsageEvent {
        event_uid: source_uid,
        source_document_id: doc_id,
        session_uid,
        occurred_at_utc,
        model: value
            .get("model")
            .or_else(|| value.pointer("/payload/model"))
            .or_else(|| value.pointer("/payload/info/model"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        mode: value
            .get("mode")
            .or_else(|| value.pointer("/payload/mode"))
            .or_else(|| value.pointer("/payload/info/mode"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        input_tokens: input.unwrap_or(0),
        output_tokens: output.unwrap_or(0),
        cache_read_tokens: cache_read.unwrap_or(0),
        cache_write_tokens: cache_write.unwrap_or(0),
        total_tokens: total,
        raw_event_kind: event_kind.to_string(),
        confidence: "high".to_string(),
    }))
}

fn number_at(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| value.get(*key))
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_u64().and_then(|n| i64::try_from(n).ok()))
        })
}

fn parse_utc(input: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(input)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn hash_text(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn local_coverage(summary: &ImportRunSummary) -> (i64, &'static str, &'static str, Vec<String>) {
    if summary.events_imported == 0 {
        return (
            0,
            "unavailable",
            "No parseable local Codex history token events were found.",
            vec![
                "local usage events".to_string(),
                "parseable token fields".to_string(),
            ],
        );
    }
    if !summary.warnings.is_empty() {
        return (
            72,
            "medium",
            "Local history imported with warnings; unknown shapes lower source coverage.",
            vec!["all event shapes parseable".to_string()],
        );
    }
    (
        100,
        "high",
        "Local history events include timestamps, token fields, and dedupe keys.",
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{count_usage_events, open_memory, usage_total};
    use std::io::Write;
    use tempfile::tempdir;

    fn write_jsonl(dir: &Path, name: &str, lines: &[&str]) -> PathBuf {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        path
    }

    #[test]
    fn imports_jsonl_token_count_events() {
        let dir = tempdir().unwrap();
        write_jsonl(
            dir.path(),
            "history.jsonl",
            &[
                r#"{"id":"event-1","type":"token_count","timestamp":"2026-07-02T18:00:00Z","session_id":"s1","model":"gpt-5.5","mode":"executor","usage":{"input_tokens":10,"output_tokens":20,"cache_read_tokens":5,"cache_write_tokens":1}}"#,
                r#"{"id":"event-2","type":"token_count","timestamp":"2026-07-02T18:05:00Z","session_id":"s1","usage":{"total_tokens":100}}"#,
            ],
        );
        let conn = open_memory().unwrap();
        let summary = LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();

        assert_eq!(summary.files_seen, 1);
        assert_eq!(summary.events_imported, 2);
        assert_eq!(count_usage_events(&conn).unwrap(), 2);
        assert_eq!(usage_total(&conn).unwrap(), 136);
    }

    #[test]
    fn imports_nested_codex_token_count_events() {
        let dir = tempdir().unwrap();
        write_jsonl(
            dir.path(),
            "rollout-2026-07-02T18-00-00.jsonl",
            &[
                r#"{"timestamp":"2026-07-02T18:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":10,"cached_input_tokens":4,"output_tokens":3,"reasoning_output_tokens":2,"total_tokens":19},"model":"gpt-5.5"},"mode":"executor"}}"#,
            ],
        );
        let conn = open_memory().unwrap();
        let summary = LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();

        assert_eq!(summary.events_imported, 1);
        assert_eq!(count_usage_events(&conn).unwrap(), 1);
        assert_eq!(usage_total(&conn).unwrap(), 19);
    }

    #[test]
    fn skips_unknown_jsonl_shapes_with_warning() {
        let dir = tempdir().unwrap();
        write_jsonl(
            dir.path(),
            "history.jsonl",
            &[
                r#"{"id":"event-1","type":"token_count","timestamp":"2026-07-02T18:00:00Z","usage":{"total_tokens":100}}"#,
                r#"{"type":"message","timestamp":"2026-07-02T18:01:00Z","text":"no tokens"}"#,
            ],
        );
        let conn = open_memory().unwrap();
        let summary = LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();

        assert_eq!(summary.events_seen, 2);
        assert_eq!(summary.events_imported, 1);
        assert_eq!(summary.warnings.len(), 1);
    }

    #[test]
    fn deduplicates_reimported_events() {
        let dir = tempdir().unwrap();
        write_jsonl(
            dir.path(),
            "history.jsonl",
            &[
                r#"{"id":"event-1","type":"token_count","timestamp":"2026-07-02T18:00:00Z","usage":{"total_tokens":100}}"#,
            ],
        );
        let conn = open_memory().unwrap();
        let importer = LocalHistoryImporter::new(vec![dir.path().to_path_buf()]);
        assert_eq!(importer.import_into(&conn).unwrap().events_imported, 1);
        assert_eq!(importer.import_into(&conn).unwrap().events_imported, 0);
        assert_eq!(count_usage_events(&conn).unwrap(), 1);
    }

    #[test]
    fn tracks_source_document_offsets_or_hashes() {
        let dir = tempdir().unwrap();
        let path = write_jsonl(
            dir.path(),
            "history.jsonl",
            &[
                r#"{"id":"event-1","timestamp":"2026-07-02T18:00:00Z","usage":{"total_tokens":100}}"#,
            ],
        );
        let conn = open_memory().unwrap();
        LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();
        let offset: i64 = conn
            .query_row(
                "SELECT last_offset FROM source_documents WHERE safe_label = 'history.jsonl'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(offset, fs::read_to_string(path).unwrap().len() as i64);
    }
}
