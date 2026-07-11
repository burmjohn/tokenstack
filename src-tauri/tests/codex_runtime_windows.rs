use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
#[cfg(windows)]
use tokenstack_lib::codex_runtime::{
    discover_codex_runtimes_in, select_codex_runtime, CodexRuntimeSettings, CodexRuntimeSource,
    RuntimeDiscoveryContext,
};

fn compile_fake_codex(directory: &Path, scenario: &str) -> PathBuf {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/fake_codex.rs");
    let executable = directory.join(if cfg!(windows) {
        format!("fake codex {scenario}.exe")
    } else {
        format!("fake codex {scenario}")
    });
    let output = Command::new("rustc")
        .arg("--edition=2021")
        .arg(source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("rustc must be available while running Rust tests");
    assert!(
        output.status.success(),
        "fake Codex compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

#[test]
fn compiled_fake_codex_spawns_directly_from_a_path_with_spaces() {
    let directory = tempfile::Builder::new()
        .prefix("tokenstack fake runtime ")
        .tempdir()
        .unwrap();
    let executable = compile_fake_codex(directory.path(), "happy");
    let mut child = Command::new(&executable)
        .args(["app-server", "--listen", "stdio://", "-c", "mcp_servers={}"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("native fake executable should spawn without a shell");

    child.stdin.as_mut().unwrap().write_all(
        br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{"experimentalApi":true}}}
"#,
    ).unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#""id":1"#));
    assert!(stdout.contains("fake-codex"));
}

#[cfg(windows)]
#[test]
fn windows_discovery_validates_thin_path_app_standalone_and_stale_configured() {
    let root = tempfile::Builder::new()
        .prefix("runtime discovery spaces ")
        .tempdir()
        .unwrap();
    let fake = compile_fake_codex(root.path(), "happy");
    let path_dir = root.path().join("thin path");
    let local = root.path().join("Local App Data");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(local.join("OpenAI/Codex/bin")).unwrap();
    fs::create_dir_all(local.join("Programs/Codex")).unwrap();
    fs::copy(&fake, path_dir.join("codex.exe")).unwrap();
    fs::copy(&fake, local.join("OpenAI/Codex/bin/codex.exe")).unwrap();
    fs::copy(&fake, local.join("Programs/Codex/codex.exe")).unwrap();
    let stale = tokenstack_lib::codex_runtime::CodexLaunchSpec {
        executable_path: root.path().join("stale.exe"),
        argv_prefix: Vec::new(),
    };
    let context = RuntimeDiscoveryContext::isolated(&[("LOCALAPPDATA", &local)], vec![path_dir]);
    let candidates = discover_codex_runtimes_in(
        &CodexRuntimeSettings {
            configured_runtime: Some(stale),
        },
        &context,
    );
    assert_eq!(candidates[0].source, CodexRuntimeSource::Configured);
    assert_eq!(
        select_codex_runtime(&candidates),
        Some(&candidates[1].launch)
    );
    assert!(candidates.iter().any(
        |value| value.source == CodexRuntimeSource::CodexApp && value.executable == Some(true)
    ));
    assert!(candidates
        .iter()
        .any(|value| value.source == CodexRuntimeSource::Standalone
            && value.executable == Some(true)));
}

#[cfg(windows)]
#[test]
fn windows_validation_terminates_hung_native_runtime() {
    let directory = tempfile::Builder::new()
        .prefix("hung runtime spaces ")
        .tempdir()
        .unwrap();
    let executable = compile_fake_codex(directory.path(), "hung_version");
    let context =
        RuntimeDiscoveryContext::isolated(&[("TOKENSTACK_CODEX_BIN", &executable)], Vec::new());
    let candidates = discover_codex_runtimes_in(&CodexRuntimeSettings::default(), &context);
    let candidate = &candidates[0];
    assert_eq!(candidate.executable, Some(false));
    assert!(candidate
        .validation_error
        .as_deref()
        .is_some_and(|value| value.contains("timed out")));
}

#[cfg(windows)]
#[test]
fn windows_npm_shim_uses_native_node_and_fixed_entrypoint() {
    let root = tempfile::Builder::new()
        .prefix("npm runtime spaces ")
        .tempdir()
        .unwrap();
    let fake = compile_fake_codex(root.path(), "happy");
    let appdata = root.path().join("Roaming Data");
    let npm = appdata.join("npm");
    let entrypoint = npm.join("node_modules/@openai/codex/bin/codex.js");
    fs::create_dir_all(entrypoint.parent().unwrap()).unwrap();
    fs::copy(fake, npm.join("node.exe")).unwrap();
    fs::write(&entrypoint, "fixture").unwrap();
    fs::write(
        npm.join("codex.cmd"),
        "@\"%dp0%\\node.exe\" \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*\n",
    )
    .unwrap();
    let context = RuntimeDiscoveryContext::isolated(&[("APPDATA", &appdata)], Vec::new());
    let candidates = discover_codex_runtimes_in(&CodexRuntimeSettings::default(), &context);
    let candidate = candidates
        .iter()
        .find(|value| value.source == CodexRuntimeSource::Npm)
        .unwrap();
    assert_eq!(candidate.launch.executable_path, npm.join("node.exe"));
    assert_eq!(
        candidate.launch.argv_prefix,
        vec![entrypoint.to_string_lossy()]
    );
    assert_eq!(candidate.executable, Some(true));
}

#[test]
fn production_sources_do_not_contain_the_reset_credit_consume_route() {
    let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let forbidden = ["account/rateLimitResetCredit", "consume"].join("/");
    let mut violations = Vec::new();
    scan_production_files(&repository_root, &forbidden, &mut violations);
    assert!(
        violations.is_empty(),
        "forbidden mutating account method found in production source: {}",
        violations.join(", ")
    );
}

fn scan_production_files(directory: &Path, forbidden: &str, violations: &mut Vec<String>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if matches!(
                name,
                ".agents"
                    | ".codebase-memory"
                    | ".git"
                    | ".omx"
                    | ".worktrees"
                    | "docs"
                    | "tests"
                    | "target"
                    | "node_modules"
                    | "dist"
            ) {
                continue;
            }
            scan_production_files(&path, forbidden, violations);
        } else if is_relevant_production_file(&path)
            && fs::read_to_string(&path)
                .map(|source| source.contains(forbidden))
                .unwrap_or(false)
        {
            violations.push(path.display().to_string());
        }
    }
}

fn is_relevant_production_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("rs" | "ts" | "tsx" | "js" | "jsx" | "json" | "toml" | "yml" | "yaml")
    )
}
