use crate::db::{
    insert_import_run, insert_usage_event, record_source_coverage, upsert_source_document,
    ImportRunSummary, UsageEvent,
};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
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
            warning_count: 0,
            warnings: Vec::new(),
        };
        let mut parseable_events = 0;
        let mut parsed_events = Vec::new();

        for file in self.discover_files()? {
            summary.files_seen += 1;
            let bytes = match fs::read(&file) {
                Ok(bytes) => bytes,
                Err(_) => {
                    record_warning(&mut summary, "local source unreadable".to_string());
                    continue;
                }
            };
            let path_hash = hash_text(&file.to_string_lossy());
            let content_hash = hash_bytes(&bytes);
            let safe_label = file
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("history.jsonl");
            let lines = if is_supported_sqlite_path(&file) {
                match read_supported_sqlite_events(&file) {
                    Ok(lines) => lines,
                    Err(_) => {
                        record_warning(
                            &mut summary,
                            "state_5.sqlite: unreadable or unsupported database".to_string(),
                        );
                        Vec::new()
                    }
                }
            } else {
                match String::from_utf8(bytes.clone()) {
                    Ok(text) => text.lines().map(ToString::to_string).collect(),
                    Err(_) => {
                        record_warning(&mut summary, "local JSONL is not UTF-8".to_string());
                        Vec::new()
                    }
                }
            };
            let doc_id = upsert_source_document(
                conn,
                "local-codex-history",
                &path_hash,
                safe_label,
                &content_hash,
                bytes.len() as i64,
            )?;

            for (line_index, line) in lines.iter().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                summary.events_seen += 1;
                match parse_event(line, doc_id, &path_hash, line_index) {
                    Ok(Some(parsed)) => {
                        parseable_events += 1;
                        parsed_events.push(parsed);
                    }
                    Ok(None) => record_warning(
                        &mut summary,
                        unknown_event_shape_warning(safe_label, line_index, line),
                    ),
                    Err(_) => record_warning(
                        &mut summary,
                        format!("{safe_label}:{} invalid event shape", line_index + 1),
                    ),
                }
            }
        }

        let mut semantic_groups: HashMap<String, Vec<ParsedEvent>> = HashMap::new();
        for parsed in parsed_events {
            semantic_groups
                .entry(parsed.semantic_dedup_key.clone())
                .or_default()
                .push(parsed);
        }
        let mut parsed_events = Vec::new();
        for (_, group) in semantic_groups {
            let mut counts: HashMap<i64, usize> = HashMap::new();
            for parsed in &group {
                *counts.entry(parsed.event.source_document_id).or_default() += 1;
            }
            let selected_document = counts
                .into_iter()
                .max_by(|(left_doc, left_count), (right_doc, right_count)| {
                    left_count
                        .cmp(right_count)
                        .then_with(|| right_doc.cmp(left_doc))
                })
                .map(|(document, _)| document)
                .unwrap_or_default();
            parsed_events.extend(
                group
                    .into_iter()
                    .filter(|parsed| parsed.event.source_document_id == selected_document),
            );
        }
        parsed_events.sort_by(|left, right| {
            left.event
                .session_uid
                .cmp(&right.event.session_uid)
                .then(left.event.occurred_at_utc.cmp(&right.event.occurred_at_utc))
                .then(left.event.event_uid.cmp(&right.event.event_uid))
        });
        let mut cumulative_by_session: HashMap<String, TokenCounters> = HashMap::new();
        for mut parsed in parsed_events {
            if parsed.cumulative {
                let previous = cumulative_by_session
                    .entry(parsed.event.session_uid.clone())
                    .or_default();
                parsed.event.input_tokens =
                    nonnegative_delta(parsed.event.input_tokens, previous.input);
                parsed.event.output_tokens =
                    nonnegative_delta(parsed.event.output_tokens, previous.output);
                parsed.event.cache_read_tokens =
                    nonnegative_delta(parsed.event.cache_read_tokens, previous.cache_read);
                parsed.event.cache_write_tokens =
                    nonnegative_delta(parsed.event.cache_write_tokens, previous.cache_write);
                parsed.event.total_tokens =
                    nonnegative_delta(parsed.event.total_tokens, previous.total);
                *previous = parsed.raw_counters;
            }
            if parsed.event.total_tokens > 0 && insert_usage_event(conn, &parsed.event)? {
                summary.events_imported += 1;
            }
        }

        let (coverage, confidence, explanation, missing) =
            local_coverage(&summary, parseable_events);
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
            collect_sources(root, &mut files)?;
        }
        files.sort();
        Ok(files)
    }
}

fn collect_sources(root: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    let canonical_root = root.canonicalize()?;
    let mut visited = HashSet::new();
    collect_bounded(&canonical_root, &canonical_root, 0, &mut visited, files)
}

fn collect_bounded(
    root: &Path,
    path: &Path,
    depth: usize,
    visited: &mut HashSet<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    const MAX_DEPTH: usize = 16;
    const MAX_FILES: usize = 4096;
    if depth > MAX_DEPTH || files.len() >= MAX_FILES {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root) {
        return Ok(());
    }
    if metadata.is_file() {
        if canonical.extension().and_then(|ext| ext.to_str()) == Some("jsonl")
            || is_supported_sqlite_path(&canonical)
        {
            files.push(canonical);
        }
        return Ok(());
    }
    if !visited.insert(canonical.clone()) {
        return Ok(());
    }
    for entry in fs::read_dir(canonical)? {
        collect_bounded(root, &entry?.path(), depth + 1, visited, files)?;
        if files.len() >= MAX_FILES {
            break;
        }
    }
    Ok(())
}

fn is_supported_sqlite_path(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("state_5.sqlite")
}

fn read_supported_sqlite_events(path: &Path) -> anyhow::Result<Vec<String>> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    const TABLES: [&str; 3] = ["events", "token_events", "session_events"];
    const JSON_COLUMNS: [&str; 4] = ["payload_json", "event_json", "data_json", "json"];
    let mut events = Vec::new();
    let mut supported_shape = false;

    for table in TABLES {
        let table_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table],
            |row| row.get(0),
        )?;
        if !table_exists {
            continue;
        }
        let pragma = format!("PRAGMA table_info({table})");
        let mut statement = conn.prepare(&pragma)?;
        let columns: Vec<String> = statement
            .query_map([], |row| row.get(1))?
            .collect::<rusqlite::Result<_>>()?;
        for column in JSON_COLUMNS {
            if !columns.iter().any(|candidate| candidate == column) {
                continue;
            }
            supported_shape = true;
            let query = format!("SELECT {column} FROM {table} WHERE {column} IS NOT NULL");
            let mut statement = conn.prepare(&query)?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            events.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
        }
    }

    let _ = supported_shape;
    Ok(events)
}

fn record_warning(summary: &mut ImportRunSummary, warning: String) {
    summary.warning_count += 1;
    if summary.warnings.len() < 20 {
        summary.warnings.push(warning);
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TokenCounters {
    input: i64,
    output: i64,
    cache_read: i64,
    cache_write: i64,
    total: i64,
}

struct ParsedEvent {
    event: UsageEvent,
    raw_counters: TokenCounters,
    cumulative: bool,
    semantic_dedup_key: String,
}

fn parse_event(
    line: &str,
    doc_id: i64,
    path_hash: &str,
    _line_index: usize,
) -> anyhow::Result<Option<ParsedEvent>> {
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

    let cumulative_usage = value
        .pointer("/payload/info/total_token_usage")
        .or_else(|| value.pointer("/payload/info/totalTokenUsage"));
    let usage = value
        .get("usage")
        .or_else(|| value.pointer("/payload/usage"))
        .or_else(|| value.pointer("/payload/info/last_token_usage"))
        .or_else(|| value.pointer("/payload/info/lastTokenUsage"))
        .or_else(|| value.pointer("/payload/info/total_token_usage"))
        .or_else(|| value.pointer("/payload/info/totalTokenUsage"))
        .or_else(|| value.pointer("/payload/info"))
        .unwrap_or(&value);
    let input = number_at(
        usage,
        &["input_tokens", "inputTokens", "input", "prompt_tokens"],
    );
    let output = number_at(
        usage,
        &[
            "output_tokens",
            "outputTokens",
            "output",
            "completion_tokens",
        ],
    );
    let cache_read = number_at(
        usage,
        &[
            "cache_read_tokens",
            "cacheReadTokens",
            "cached_input_tokens",
            "cachedInputTokens",
        ],
    );
    let cache_write = number_at(usage, &["cache_write_tokens", "cacheWriteTokens"]);
    let reasoning_output = number_at(usage, &["reasoning_output_tokens", "reasoningOutputTokens"]);
    let explicit_total = number_at(usage, &["total_tokens", "totalTokens", "tokens"]);
    let total = explicit_total.unwrap_or_else(|| {
        input.unwrap_or(0)
            + output.unwrap_or(0)
            + cache_read.unwrap_or(0)
            + cache_write.unwrap_or(0)
            + reasoning_output.unwrap_or(0)
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

    let turn_uid = value
        .get("turn_id")
        .or_else(|| value.get("turnId"))
        .or_else(|| value.pointer("/payload/turn_id"))
        .or_else(|| value.pointer("/payload/turnId"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let canonical_identity = format!(
        "{event_kind}|{session_uid}|{turn_uid}|{}|{}|{}|{}|{}|{}",
        occurred_at_utc.to_rfc3339(),
        input.unwrap_or(0),
        output.unwrap_or(0),
        cache_read.unwrap_or(0),
        cache_write.unwrap_or(0),
        total
    );
    let source_identity = value
        .get("id")
        .or_else(|| value.get("event_id"))
        .and_then(Value::as_str)
        .map(|id| format!("id:{id}"))
        .unwrap_or_else(|| {
            let canonical_payload =
                serde_json::to_string(&value).unwrap_or_else(|_| canonical_identity.clone());
            format!("payload:{}", hash_text(&canonical_payload))
        });
    let source_uid = hash_text(&format!("{path_hash}|{source_identity}"));

    let event = UsageEvent {
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
    };
    Ok(Some(ParsedEvent {
        raw_counters: TokenCounters {
            input: event.input_tokens,
            output: event.output_tokens,
            cache_read: event.cache_read_tokens,
            cache_write: event.cache_write_tokens,
            total: event.total_tokens,
        },
        event,
        cumulative: cumulative_usage.is_some(),
        semantic_dedup_key: hash_text(&canonical_identity),
    }))
}

fn nonnegative_delta(current: i64, previous: i64) -> i64 {
    if current < previous {
        current.max(0)
    } else {
        current - previous
    }
}

fn unknown_event_shape_warning(safe_label: &str, line_index: usize, line: &str) -> String {
    let shape = serde_json::from_str::<Value>(line)
        .map(|value| summarize_unknown_shape(&value))
        .unwrap_or_else(|_| "invalid-json".to_string());
    format!(
        "{safe_label}:{} unknown event shape skipped ({shape})",
        line_index + 1
    )
}

fn summarize_unknown_shape(value: &Value) -> String {
    let mut parts = Vec::new();
    push_shape_string(&mut parts, "type", value.get("type"));
    push_shape_string(&mut parts, "kind", value.get("kind"));
    push_shape_string(&mut parts, "event", value.get("event"));
    push_shape_string(&mut parts, "payload.type", value.pointer("/payload/type"));
    push_shape_keys(&mut parts, "keys", value);
    if let Some(payload) = value.get("payload") {
        push_shape_keys(&mut parts, "payload.keys", payload);
    }
    if let Some(info) = value.pointer("/payload/info") {
        push_shape_keys(&mut parts, "payload.info.keys", info);
    }
    if parts.is_empty() {
        "non-object-json".to_string()
    } else {
        parts.join("; ")
    }
}

fn push_shape_string(parts: &mut Vec<String>, label: &str, value: Option<&Value>) {
    if let Some(value) = value
        .and_then(Value::as_str)
        .filter(|value| is_safe_shape_value(value))
    {
        parts.push(format!("{label}={value}"));
    }
}

fn push_shape_keys(parts: &mut Vec<String>, label: &str, value: &Value) {
    if let Some(object) = value.as_object() {
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        keys.truncate(16);
        if !keys.is_empty() {
            parts.push(format!("{label}={}", keys.join(",")));
        }
    }
}

fn is_safe_shape_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
        })
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
    hash_bytes(input.as_bytes())
}

fn hash_bytes(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

fn local_coverage(
    summary: &ImportRunSummary,
    parseable_events: usize,
) -> (i64, &'static str, &'static str, Vec<String>) {
    if parseable_events == 0 {
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
    if summary.warning_count > 0 {
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
    fn imports_codex_app_token_count_info_events() {
        let dir = tempdir().unwrap();
        write_jsonl(
            dir.path(),
            "rollout-2026-07-07T00-00-00.jsonl",
            &[
                r#"{"timestamp":"2026-07-07T00:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"input_tokens":10,"cached_input_tokens":4,"output_tokens":3,"reasoning_output_tokens":2,"total_tokens":19},"rate_limits":{"primary":{"used_percent":12.5}}}}"#,
            ],
        );
        let conn = open_memory().unwrap();
        let summary = LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();

        assert_eq!(summary.events_imported, 1);
        assert_eq!(summary.warnings, Vec::<String>::new());
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
                r#"{"type":"message","timestamp":"2026-07-02T18:01:00Z","text":"secret local text","payload":{"type":"assistant_message","content":"secret payload"}}"#,
            ],
        );
        let conn = open_memory().unwrap();
        let summary = LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();

        assert_eq!(summary.events_seen, 2);
        assert_eq!(summary.events_imported, 1);
        assert_eq!(summary.warnings.len(), 1);
        assert!(summary.warnings[0].contains("type=message"));
        assert!(summary.warnings[0].contains("payload.type=assistant_message"));
        assert!(summary.warnings[0].contains("keys=payload,text,timestamp,type"));
        assert!(!summary.warnings[0].contains("secret local text"));
        assert!(!summary.warnings[0].contains("secret payload"));
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
    fn reimported_duplicate_events_keep_local_coverage_available() {
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
        importer.import_into(&conn).unwrap();
        let summary = importer.import_into(&conn).unwrap();

        let (coverage_percent, confidence): (i64, String) = conn
            .query_row(
                "SELECT coverage_percent, confidence FROM source_coverage WHERE metric_key = 'local-usage' ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(summary.events_imported, 0);
        assert_eq!(coverage_percent, 100);
        assert_eq!(confidence, "high");
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

    #[test]
    fn cumulative_snapshots_are_imported_as_nonnegative_session_deltas() {
        let dir = tempdir().unwrap();
        write_jsonl(
            dir.path(),
            "rollout.jsonl",
            &[
                r#"{"timestamp":"2026-07-02T18:00:00Z","type":"event_msg","payload":{"type":"token_count","session_id":"shared-session","info":{"total_token_usage":{"total_tokens":100}}}}"#,
                r#"{"timestamp":"2026-07-02T18:05:00Z","type":"event_msg","payload":{"type":"token_count","session_id":"shared-session","info":{"total_token_usage":{"total_tokens":150}}}}"#,
            ],
        );
        let conn = open_memory().unwrap();

        LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();

        assert_eq!(count_usage_events(&conn).unwrap(), 2);
        assert_eq!(usage_total(&conn).unwrap(), 150);
    }

    #[test]
    fn duplicate_app_and_cli_views_of_one_event_are_imported_once() {
        let dir = tempdir().unwrap();
        let app = dir.path().join("app");
        let cli = dir.path().join("cli");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&cli).unwrap();
        let event = r#"{"timestamp":"2026-07-02T18:00:00Z","type":"event_msg","payload":{"type":"token_count","session_id":"shared-session","turn_id":"turn-1","info":{"last_token_usage":{"input_tokens":40,"output_tokens":10,"total_tokens":50}}}}"#;
        write_jsonl(&app, "rollout.jsonl", &[event]);
        write_jsonl(&cli, "rollout.jsonl", &[event]);
        let conn = open_memory().unwrap();

        let summary = LocalHistoryImporter::new(vec![app, cli])
            .import_into(&conn)
            .unwrap();

        assert_eq!(summary.events_seen, 2);
        assert_eq!(summary.events_imported, 1);
        assert_eq!(count_usage_events(&conn).unwrap(), 1);
        assert_eq!(usage_total(&conn).unwrap(), 50);
    }

    #[test]
    fn distinct_incremental_turn_usage_is_additive_and_reimport_is_idempotent() {
        let dir = tempdir().unwrap();
        write_jsonl(
            dir.path(),
            "rollout.jsonl",
            &[
                r#"{"timestamp":"2026-07-02T18:00:00Z","type":"event_msg","payload":{"type":"token_count","sessionId":"s1","turnId":"t1","info":{"lastTokenUsage":{"totalTokens":50}}}}"#,
                r#"{"timestamp":"2026-07-02T18:05:00Z","type":"event_msg","payload":{"type":"token_count","sessionId":"s1","turnId":"t2","info":{"lastTokenUsage":{"totalTokens":70}}}}"#,
            ],
        );
        let conn = open_memory().unwrap();
        let importer = LocalHistoryImporter::new(vec![dir.path().to_path_buf()]);

        assert_eq!(importer.import_into(&conn).unwrap().events_imported, 2);
        assert_eq!(usage_total(&conn).unwrap(), 120);
        assert_eq!(importer.import_into(&conn).unwrap().events_imported, 0);
        assert_eq!(usage_total(&conn).unwrap(), 120);
    }

    #[test]
    fn warning_samples_are_bounded_and_never_copy_content_values() {
        let dir = tempdir().unwrap();
        let secret = "private prompt body that must never enter diagnostics";
        let lines: Vec<String> = (0..80)
            .map(|index| format!(r#"{{"type":"message","timestamp":"2026-07-02T18:00:00Z","payload":{{"type":"assistant_message","content":"{secret}-{index}"}}}}"#))
            .collect();
        let line_refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        write_jsonl(dir.path(), "rollout.jsonl", &line_refs);
        let conn = open_memory().unwrap();

        let summary = LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();

        assert!(summary.warnings.len() <= 20);
        assert!(summary
            .warnings
            .iter()
            .all(|warning| !warning.contains(secret)));
    }

    #[test]
    fn imports_archived_session_jsonl() {
        let home = tempdir().unwrap();
        let archive = home.path().join("archived_sessions");
        fs::create_dir_all(&archive).unwrap();
        write_jsonl(
            &archive,
            "rollout-archived.jsonl",
            &[
                r#"{"timestamp":"2026-06-01T12:00:00Z","type":"event_msg","payload":{"type":"token_count","session_id":"archived-s1","info":{"last_token_usage":{"total_tokens":33}}}}"#,
            ],
        );
        let conn = open_memory().unwrap();

        LocalHistoryImporter::new(vec![archive])
            .import_into(&conn)
            .unwrap();

        assert_eq!(count_usage_events(&conn).unwrap(), 1);
        assert_eq!(usage_total(&conn).unwrap(), 33);
    }

    #[test]
    fn imports_known_sqlite_json_payload_and_deduplicates_shared_jsonl_event() {
        let dir = tempdir().unwrap();
        let event = r#"{"timestamp":"2026-07-02T18:00:00Z","type":"event_msg","payload":{"type":"token_count","session_id":"shared-s1","turn_id":"turn-1","info":{"last_token_usage":{"input_tokens":40,"output_tokens":10,"total_tokens":50}}}}"#;
        write_jsonl(dir.path(), "rollout.jsonl", &[event]);
        let sqlite_path = dir.path().join("state_5.sqlite");
        let source = Connection::open(&sqlite_path).unwrap();
        source
            .execute_batch("CREATE TABLE events (id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);")
            .unwrap();
        source
            .execute(
                "INSERT INTO events (id, payload_json) VALUES (?1, ?2)",
                rusqlite::params!["event-1", event],
            )
            .unwrap();
        drop(source);
        let conn = open_memory().unwrap();

        let summary = LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();

        assert_eq!(summary.files_seen, 2);
        assert_eq!(summary.events_seen, 2);
        assert_eq!(summary.events_imported, 1);
        assert_eq!(count_usage_events(&conn).unwrap(), 1);
        assert_eq!(usage_total(&conn).unwrap(), 50);
    }

    #[test]
    fn sqlite_unknown_payload_columns_produce_shape_only_warning() {
        let dir = tempdir().unwrap();
        let sqlite_path = dir.path().join("state_5.sqlite");
        let source = Connection::open(&sqlite_path).unwrap();
        source
            .execute_batch("CREATE TABLE private_messages (prompt_body TEXT NOT NULL);")
            .unwrap();
        source
            .execute(
                "INSERT INTO private_messages (prompt_body) VALUES (?1)",
                ["secret prompt value"],
            )
            .unwrap();
        drop(source);
        let conn = open_memory().unwrap();

        let summary = LocalHistoryImporter::new(vec![sqlite_path])
            .import_into(&conn)
            .unwrap();

        assert_eq!(summary.events_imported, 0);
        assert!(summary.warnings.iter().all(|warning| {
            !warning.contains("secret prompt value") && !warning.contains("prompt_body")
        }));
    }

    #[test]
    fn cumulative_events_are_normalized_before_timestamp_ordering() {
        let dir = tempdir().unwrap();
        write_jsonl(
            dir.path(),
            "z-older.jsonl",
            &[
                r#"{"id":"older","timestamp":"2026-07-02T18:00:00Z","payload":{"type":"token_count","session_id":"s1","info":{"total_token_usage":{"total_tokens":100}}}}"#,
            ],
        );
        write_jsonl(
            dir.path(),
            "a-newer.jsonl",
            &[
                r#"{"id":"newer","timestamp":"2026-07-02T18:05:00Z","payload":{"type":"token_count","session_id":"s1","info":{"total_token_usage":{"total_tokens":150}}}}"#,
            ],
        );
        let conn = open_memory().unwrap();
        LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();
        assert_eq!(usage_total(&conn).unwrap(), 150);
    }

    #[test]
    fn distinct_explicit_ids_prevent_same_timestamp_counter_collision() {
        let dir = tempdir().unwrap();
        write_jsonl(
            dir.path(),
            "rollout.jsonl",
            &[
                r#"{"id":"event-a","timestamp":"2026-07-02T18:00:00Z","session_id":"s1","usage":{"total_tokens":50}}"#,
                r#"{"id":"event-b","timestamp":"2026-07-02T18:00:00Z","session_id":"s1","usage":{"total_tokens":50}}"#,
            ],
        );
        let conn = open_memory().unwrap();
        LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();
        assert_eq!(count_usage_events(&conn).unwrap(), 2);
        assert_eq!(usage_total(&conn).unwrap(), 100);
    }

    #[test]
    fn warning_count_tracks_all_warnings_while_samples_remain_bounded() {
        let dir = tempdir().unwrap();
        let lines: Vec<String> = (0..80)
            .map(|_| r#"{"type":"message","payload":{"type":"assistant_message"}}"#.to_string())
            .collect();
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        write_jsonl(dir.path(), "rollout.jsonl", &refs);
        let conn = open_memory().unwrap();
        let summary = LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();
        assert_eq!(summary.warning_count, 80);
        assert_eq!(summary.warnings.len(), 20);
    }

    #[cfg(unix)]
    #[test]
    fn recursive_discovery_does_not_follow_symlink_escape() {
        use std::os::unix::fs::symlink;
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write_jsonl(
            outside.path(),
            "outside.jsonl",
            &[
                r#"{"id":"outside","timestamp":"2026-07-02T18:00:00Z","usage":{"total_tokens":999}}"#,
            ],
        );
        symlink(outside.path(), root.path().join("escaped")).unwrap();
        let conn = open_memory().unwrap();
        LocalHistoryImporter::new(vec![root.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();
        assert_eq!(count_usage_events(&conn).unwrap(), 0);
    }

    #[test]
    fn cumulative_counter_decrease_resets_baseline_without_overcounting() {
        let dir = tempdir().unwrap();
        write_jsonl(
            dir.path(),
            "rollout.jsonl",
            &[
                r#"{"id":"a","timestamp":"2026-07-02T18:00:00Z","payload":{"type":"token_count","session_id":"s1","info":{"total_token_usage":{"total_tokens":100}}}}"#,
                r#"{"id":"b","timestamp":"2026-07-02T18:01:00Z","payload":{"type":"token_count","session_id":"s1","info":{"total_token_usage":{"total_tokens":150}}}}"#,
                r#"{"id":"c","timestamp":"2026-07-02T18:02:00Z","payload":{"type":"token_count","session_id":"s1","info":{"total_token_usage":{"total_tokens":20}}}}"#,
                r#"{"id":"d","timestamp":"2026-07-02T18:03:00Z","payload":{"type":"token_count","session_id":"s1","info":{"total_token_usage":{"total_tokens":30}}}}"#,
            ],
        );
        let conn = open_memory().unwrap();
        LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();
        assert_eq!(usage_total(&conn).unwrap(), 180);
    }

    #[test]
    fn corrupt_state_database_warns_and_unrelated_db_is_ignored() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("state_5.sqlite"), b"not sqlite").unwrap();
        fs::write(dir.path().join("unrelated.db"), b"private unrelated bytes").unwrap();
        let conn = open_memory().unwrap();
        let summary = LocalHistoryImporter::new(vec![dir.path().to_path_buf()])
            .import_into(&conn)
            .unwrap();
        assert_eq!(summary.files_seen, 1);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(count_usage_events(&conn).unwrap(), 0);
    }

    #[test]
    fn sqlite_import_is_read_only_and_creates_no_sidecars() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state_5.sqlite");
        let source = Connection::open(&path).unwrap();
        source
            .execute_batch("CREATE TABLE events (payload_json TEXT NOT NULL);")
            .unwrap();
        source
            .execute(
                "INSERT INTO events VALUES (?1)",
                [r#"{"id":"a","timestamp":"2026-07-02T18:00:00Z","usage":{"total_tokens":10}}"#],
            )
            .unwrap();
        drop(source);
        let before: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        let conn = open_memory().unwrap();
        LocalHistoryImporter::new(vec![path])
            .import_into(&conn)
            .unwrap();
        let after: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(before, after);
    }

    #[test]
    fn differing_app_cli_ids_and_envelopes_alias_by_semantic_usage() {
        let dir = tempdir().unwrap();
        let app = dir.path().join("app");
        let cli = dir.path().join("cli");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&cli).unwrap();
        write_jsonl(
            &app,
            "rollout.jsonl",
            &[
                r#"{"id":"app-event","timestamp":"2026-07-02T18:00:00Z","type":"event_msg","payload":{"type":"token_count","session_id":"s1","turn_id":"t1","info":{"last_token_usage":{"input_tokens":40,"output_tokens":10,"total_tokens":50}},"app_extra":"ignored"}}"#,
            ],
        );
        write_jsonl(
            &cli,
            "rollout.jsonl",
            &[
                r#"{"event_id":"cli-event","timestamp":"2026-07-02T18:00:00+00:00","type":"token_count","session_id":"s1","turn_id":"t1","usage":{"input_tokens":40,"output_tokens":10,"total_tokens":50},"cli_extra":{"shape":true}}"#,
            ],
        );
        let conn = open_memory().unwrap();
        LocalHistoryImporter::new(vec![app, cli])
            .import_into(&conn)
            .unwrap();
        assert_eq!(count_usage_events(&conn).unwrap(), 1);
        assert_eq!(usage_total(&conn).unwrap(), 50);
    }
}
