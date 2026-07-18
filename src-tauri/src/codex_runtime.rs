#![allow(clippy::result_large_err)]

use crate::codex_app_server::{
    AccountConnectorError, AccountConnectorErrorKind, AccountLaunchDiagnostics,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexLaunchSpec {
    pub executable_path: PathBuf,
    pub argv_prefix: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CodexRuntimeSettings {
    pub configured_runtime: Option<CodexLaunchSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexRuntimeSource {
    Configured,
    Environment,
    Path,
    CodexApp,
    Npm,
    Standalone,
    Msix,
}

#[derive(Debug, Clone)]
pub struct CodexRuntimeCandidate {
    pub display_path: PathBuf,
    pub launch: CodexLaunchSpec,
    pub source: CodexRuntimeSource,
    pub exists: bool,
    pub executable: Option<bool>,
    pub version: Option<String>,
    pub validation_error: Option<String>,
    pub validation: Option<AccountConnectorError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexRuntimeValidation {
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeDiscoveryContext {
    env: HashMap<OsString, OsString>,
    path_dirs: Vec<PathBuf>,
    validation_timeout: Duration,
}

impl RuntimeDiscoveryContext {
    pub fn from_current_process() -> Self {
        let env = env::vars_os().collect::<HashMap<_, _>>();
        let path_dirs = environment_value(&env, "PATH")
            .map(env::split_paths)
            .into_iter()
            .flatten()
            .collect();
        Self {
            env,
            path_dirs,
            validation_timeout: Duration::from_secs(2),
        }
    }

    pub fn isolated(entries: &[(&str, &Path)], path_dirs: Vec<PathBuf>) -> Self {
        Self {
            env: entries
                .iter()
                .map(|(key, value)| (OsString::from(key), value.as_os_str().to_owned()))
                .collect(),
            path_dirs,
            validation_timeout: Duration::from_millis(250),
        }
    }

    fn value(&self, key: &str) -> Option<PathBuf> {
        environment_value(&self.env, key).map(PathBuf::from)
    }
}

fn environment_value<'a>(
    environment: &'a HashMap<OsString, OsString>,
    key: &str,
) -> Option<&'a OsString> {
    if let Some(value) = environment.get(OsStr::new(key)) {
        return Some(value);
    }
    #[cfg(windows)]
    {
        environment
            .iter()
            .find(|(candidate, _)| candidate.to_string_lossy().eq_ignore_ascii_case(key))
            .map(|(_, value)| value)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn discover_codex_runtimes(settings: &CodexRuntimeSettings) -> Vec<CodexRuntimeCandidate> {
    discover_codex_runtimes_with(settings, &RuntimeDiscoveryContext::from_current_process())
}

pub fn discover_codex_runtimes_in(
    settings: &CodexRuntimeSettings,
    context: &RuntimeDiscoveryContext,
) -> Vec<CodexRuntimeCandidate> {
    discover_codex_runtimes_with(settings, context)
}

pub fn validate_codex_runtime(
    spec: &CodexLaunchSpec,
) -> Result<CodexRuntimeValidation, AccountConnectorError> {
    validate_codex_runtime_with(spec, Duration::from_secs(2))
}

pub fn select_codex_runtime(candidates: &[CodexRuntimeCandidate]) -> Option<&CodexLaunchSpec> {
    candidates
        .iter()
        .find(|candidate| candidate.executable == Some(true))
        .map(|candidate| &candidate.launch)
}

pub(crate) fn discover_codex_runtimes_with(
    settings: &CodexRuntimeSettings,
    context: &RuntimeDiscoveryContext,
) -> Vec<CodexRuntimeCandidate> {
    let mut raw = Vec::new();
    let mut forced = Vec::new();
    if let Some(spec) = settings.configured_runtime.clone() {
        raw.push((
            spec.executable_path.clone(),
            spec,
            CodexRuntimeSource::Configured,
        ));
    }
    if let Some(path) = context.value("TOKENSTACK_CODEX_BIN") {
        raw.push((
            path.clone(),
            native_spec(path),
            CodexRuntimeSource::Environment,
        ));
    }
    for directory in &context.path_dirs {
        for name in native_names() {
            let path = directory.join(name);
            raw.push((path.clone(), native_spec(path), CodexRuntimeSource::Path));
        }
    }
    if let Some(local) = context.value("LOCALAPPDATA") {
        let app = local
            .join("OpenAI")
            .join("Codex")
            .join("bin")
            .join("codex.exe");
        raw.push((app.clone(), native_spec(app), CodexRuntimeSource::CodexApp));
    }
    if let Some(appdata) = context.value("APPDATA") {
        let shim = appdata.join("npm").join("codex.cmd");
        raw.push(npm_candidate(&shim, context));
    }
    if let Some(local) = context.value("LOCALAPPDATA") {
        for directory in [
            local.join("Programs").join("Codex"),
            local.join("Programs").join("OpenAI Codex"),
        ] {
            let path = directory.join("codex.exe");
            raw.push((
                path.clone(),
                native_spec(path),
                CodexRuntimeSource::Standalone,
            ));
        }
    }
    if let Some(program_files) = context.value("ProgramFiles") {
        enumerate_msix_root(&program_files.join("WindowsApps"), &mut raw, &mut forced);
    }

    let mut seen = HashSet::new();
    let mut discovered = raw
        .into_iter()
        .filter_map(|(display_path, spec, source)| {
            let key = normalized_key(&spec);
            if !seen.insert(key) {
                return None;
            }
            let metadata = fs::metadata(&spec.executable_path);
            let exists = metadata.is_ok();
            let validation = metadata
                .as_ref()
                .ok()
                .filter(|value| value.is_file())
                .map(|_| validate_codex_runtime_with(&spec, context.validation_timeout));
            let (executable, version, validation_error, structured_validation) = match validation {
                Some(Ok(value)) => (Some(true), Some(value.version), None, None),
                Some(Err(error)) => (
                    Some(false),
                    None,
                    Some(error.public_message.clone()),
                    Some(error),
                ),
                None if metadata.as_ref().is_ok_and(|value| !value.is_file()) => (
                    Some(false),
                    None,
                    Some("candidate is not an executable file".to_string()),
                    None,
                ),
                None => (
                    None,
                    None,
                    Some(
                        metadata
                            .err()
                            .map(|error| match error.kind() {
                                std::io::ErrorKind::NotFound => "file not found".to_string(),
                                std::io::ErrorKind::PermissionDenied => "access denied".to_string(),
                                _ => "candidate metadata unavailable".to_string(),
                            })
                            .unwrap_or_else(|| "file not found".to_string()),
                    ),
                    None,
                ),
            };
            Some(CodexRuntimeCandidate {
                display_path,
                launch: spec,
                source,
                exists,
                executable,
                version,
                validation_error,
                validation: structured_validation,
            })
        })
        .collect::<Vec<_>>();
    discovered.append(&mut forced);
    discovered
}

fn enumerate_msix_root(
    root: &Path,
    raw: &mut Vec<(PathBuf, CodexLaunchSpec, CodexRuntimeSource)>,
    forced: &mut Vec<CodexRuntimeCandidate>,
) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            let intended = root.join("<inaccessible-codex-package>/app/resources/codex.exe");
            let launch = native_spec(intended.clone());
            let message = "access denied";
            let mut validation = AccountConnectorError::new(
                AccountConnectorErrorKind::Spawn,
                "discover_msix",
                message,
            );
            validation.launch =
                AccountLaunchDiagnostics::validation(intended.display().to_string(), Vec::new());
            forced.push(CodexRuntimeCandidate {
                display_path: intended,
                launch,
                source: CodexRuntimeSource::Msix,
                exists: false,
                executable: Some(false),
                version: None,
                validation_error: Some(message.to_string()),
                validation: Some(validation),
            });
            return;
        }
        Err(_) => return,
    };
    let mut packages = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    packages.sort();
    for package in packages {
        let name = package
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_ascii_lowercase();
        if name.contains("codex") && (name.contains("openai") || name.starts_with("codex")) {
            let path = package.join("app").join("resources").join("codex.exe");
            raw.push((path.clone(), native_spec(path), CodexRuntimeSource::Msix));
        }
    }
}

fn native_spec(path: PathBuf) -> CodexLaunchSpec {
    CodexLaunchSpec {
        executable_path: path,
        argv_prefix: Vec::new(),
    }
}

fn native_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["codex.exe"]
    } else {
        &["codex"]
    }
}

fn npm_candidate(
    shim: &Path,
    context: &RuntimeDiscoveryContext,
) -> (PathBuf, CodexLaunchSpec, CodexRuntimeSource) {
    let unresolved = CodexLaunchSpec {
        executable_path: shim.to_path_buf(),
        argv_prefix: Vec::new(),
    };
    let Ok(contents) = fs::read_to_string(shim) else {
        return (shim.to_path_buf(), unresolved, CodexRuntimeSource::Npm);
    };
    let targets = contents
        .lines()
        .filter_map(|line| parse_standard_npm_line(shim, line))
        .collect::<Vec<_>>();
    let entrypoint = (targets.len() == 1).then(|| targets[0].clone());
    let node = shim
        .parent()
        .map(|directory| directory.join("node.exe"))
        .filter(|path| path.is_file())
        .or_else(|| {
            context
                .path_dirs
                .iter()
                .flat_map(|dir| [dir.join("node.exe"), dir.join("node")])
                .find(|path| path.is_file())
        });
    match (node, entrypoint) {
        (Some(node), Some(entrypoint)) => (
            shim.to_path_buf(),
            CodexLaunchSpec {
                executable_path: node,
                argv_prefix: vec![entrypoint.to_string_lossy().into_owned()],
            },
            CodexRuntimeSource::Npm,
        ),
        _ => (shim.to_path_buf(), unresolved, CodexRuntimeSource::Npm),
    }
}

fn parse_standard_npm_line(shim: &Path, line: &str) -> Option<PathBuf> {
    let line = line.trim();
    let valid = [
        "@\"%dp0%\\node.exe\" \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*",
        "@\"%~dp0\\node.exe\" \"%~dp0\\node_modules\\@openai\\codex\\bin\\codex.js\" %*",
        "endLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & \"%_prog%\"  \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*",
    ];
    valid.contains(&line).then(|| {
        shim.parent()
            .unwrap_or_else(|| Path::new("."))
            .join("node_modules/@openai/codex/bin/codex.js")
    })
}

fn normalized_key(spec: &CodexLaunchSpec) -> String {
    let path =
        fs::canonicalize(&spec.executable_path).unwrap_or_else(|_| spec.executable_path.clone());
    let rendered = path.to_string_lossy();
    let normalized = if cfg!(windows) {
        rendered.to_ascii_lowercase()
    } else {
        rendered.into_owned()
    };
    // Fixed argv is part of runnable identity: npm node + Codex JS is distinct
    // from an otherwise identical native executable path.
    format!("{}\0{}", normalized, spec.argv_prefix.join("\0"))
}

fn validate_codex_runtime_with(
    spec: &CodexLaunchSpec,
    timeout: Duration,
) -> Result<CodexRuntimeValidation, AccountConnectorError> {
    let make_error = |kind, message: &str| {
        let mut error = AccountConnectorError::new(kind, "validate_runtime", message);
        error.launch = AccountLaunchDiagnostics::validation(
            spec.executable_path.display().to_string(),
            spec.argv_prefix.clone(),
        );
        error
    };
    let mut child = Command::new(&spec.executable_path)
        .args(&spec.argv_prefix)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            make_error(
                AccountConnectorErrorKind::Spawn,
                &format!("runtime spawn failed: {error}"),
            )
        })?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().map_err(|error| {
                    make_error(
                        AccountConnectorErrorKind::Protocol,
                        &format!("runtime output unavailable: {error}"),
                    )
                })?;
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !status.success() {
                    let mut error = make_error(
                        AccountConnectorErrorKind::UnsupportedCli,
                        "runtime version check exited unsuccessfully",
                    );
                    error.exit_code = status.code();
                    return Err(error);
                }
                return Ok(CodexRuntimeValidation { version: stdout });
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let killed = child.kill().is_ok();
                let waited = child.wait().is_ok();
                let mut error = make_error(
                    AccountConnectorErrorKind::Timeout,
                    "runtime version check timed out",
                );
                error.timed_out = true;
                error.child_terminated = killed && waited;
                return Err(error);
            }
            Err(error) => {
                return Err(make_error(
                    AccountConnectorErrorKind::Protocol,
                    &format!("runtime status unavailable: {error}"),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(windows))]
    use std::io::Write;

    #[cfg(not(windows))]
    fn executable(path: &Path) {
        let mut file = fs::File::create(path).unwrap();
        use std::os::unix::fs::PermissionsExt;
        writeln!(file, "#!/bin/sh\necho codex-test").unwrap();
        let mut permissions = file.metadata().unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn configured_candidate_is_first_even_when_stale() {
        let configured = CodexLaunchSpec {
            executable_path: PathBuf::from("missing"),
            argv_prefix: vec!["fixed".into()],
        };
        let candidates = discover_codex_runtimes_with(
            &CodexRuntimeSettings {
                configured_runtime: Some(configured.clone()),
            },
            &RuntimeDiscoveryContext::isolated(&[], Vec::new()),
        );
        assert_eq!(candidates[0].launch, configured);
        assert_eq!(
            candidates[0].validation_error.as_deref(),
            Some("file not found")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn thin_path_and_app_runtime_keep_precedence_and_deduplicate() {
        let root = tempfile::tempdir().unwrap();
        let path_dir = root.path().join("thin");
        let local = root.path().join("local");
        fs::create_dir_all(&path_dir).unwrap();
        fs::create_dir_all(local.join("OpenAI/Codex/bin")).unwrap();
        executable(&path_dir.join("codex"));
        executable(&local.join("OpenAI/Codex/bin/codex.exe"));
        let context =
            RuntimeDiscoveryContext::isolated(&[("LOCALAPPDATA", &local)], vec![path_dir]);
        let candidates = discover_codex_runtimes_with(&CodexRuntimeSettings::default(), &context);
        assert_eq!(candidates[0].source, CodexRuntimeSource::Path);
        assert!(candidates
            .iter()
            .any(|candidate| candidate.source == CodexRuntimeSource::CodexApp
                && candidate.executable == Some(true)));
    }

    #[test]
    fn no_runtime_records_all_missing_candidates() {
        let root = tempfile::tempdir().unwrap();
        let local = root.path().join("local");
        let appdata = root.path().join("roaming");
        let candidates = discover_codex_runtimes_with(
            &CodexRuntimeSettings::default(),
            &RuntimeDiscoveryContext::isolated(
                &[("LOCALAPPDATA", &local), ("APPDATA", &appdata)],
                Vec::new(),
            ),
        );
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|candidate| !candidate.exists));
    }

    #[test]
    fn msix_package_resource_candidate_is_recorded_when_not_accessible() {
        let root = tempfile::tempdir().unwrap();
        let program_files = root.path().join("Program Files");
        fs::create_dir_all(program_files.join("WindowsApps/OpenAI.Codex_123")).unwrap();
        let candidates = discover_codex_runtimes_with(
            &CodexRuntimeSettings::default(),
            &RuntimeDiscoveryContext::isolated(&[("ProgramFiles", &program_files)], Vec::new()),
        );
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.source == CodexRuntimeSource::Msix)
            .unwrap();
        assert!(!candidate.exists);
        assert_eq!(
            candidate.validation_error.as_deref(),
            Some("file not found")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn standalone_runtime_is_validated_after_app_and_npm_candidates() {
        let root = tempfile::tempdir().unwrap();
        let local = root.path().join("local");
        let standalone = local.join("Programs/Codex/codex.exe");
        fs::create_dir_all(standalone.parent().unwrap()).unwrap();
        executable(&standalone);
        let candidates = discover_codex_runtimes_with(
            &CodexRuntimeSettings::default(),
            &RuntimeDiscoveryContext::isolated(&[("LOCALAPPDATA", &local)], Vec::new()),
        );
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.source == CodexRuntimeSource::Standalone)
            .unwrap();
        assert_eq!(candidate.executable, Some(true));
        assert_eq!(candidate.version.as_deref(), Some("codex-test"));
    }

    #[cfg(not(windows))]
    #[test]
    fn standard_npm_shim_resolves_to_native_node_with_fixed_prefix() {
        let root = tempfile::tempdir().unwrap();
        let appdata = root.path().join("roaming");
        let npm = appdata.join("npm");
        let node = npm.join("node.exe");
        let entrypoint = npm.join("node_modules/@openai/codex/bin/codex.js");
        fs::create_dir_all(entrypoint.parent().unwrap()).unwrap();
        executable(&node);
        fs::write(&entrypoint, "fixture").unwrap();
        fs::write(
            npm.join("codex.cmd"),
            "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\nIF EXIST \"%dp0%\\node.exe\" (\r\n  SET \"_prog=%dp0%\\node.exe\"\r\n) ELSE (\r\n  SET \"_prog=node\"\r\n)\r\nendLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & \"%_prog%\"  \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*\r\n",
        )
        .unwrap();
        let candidates = discover_codex_runtimes_with(
            &CodexRuntimeSettings::default(),
            &RuntimeDiscoveryContext::isolated(&[("APPDATA", &appdata)], Vec::new()),
        );
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.source == CodexRuntimeSource::Npm)
            .unwrap();
        assert_eq!(candidate.launch.executable_path, node);
        assert_eq!(
            candidate.launch.argv_prefix,
            vec![entrypoint.to_string_lossy()]
        );
        assert_eq!(candidate.executable, Some(true));
        assert!(candidate.validation.is_none());
    }

    #[test]
    fn arbitrary_npm_command_is_never_accepted_as_a_launch_prefix() {
        let root = tempfile::tempdir().unwrap();
        let appdata = root.path().join("roaming");
        let npm = appdata.join("npm");
        fs::create_dir_all(&npm).unwrap();
        fs::write(npm.join("codex.cmd"), "powershell -Command dangerous\n").unwrap();
        let candidates = discover_codex_runtimes_with(
            &CodexRuntimeSettings::default(),
            &RuntimeDiscoveryContext::isolated(&[("APPDATA", &appdata)], Vec::new()),
        );
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.source == CodexRuntimeSource::Npm)
            .unwrap();
        assert!(candidate.launch.argv_prefix.is_empty());
        assert_eq!(candidate.executable, Some(false));
        assert!(candidate.validation.is_some());
    }

    #[test]
    fn npm_shim_rejects_suffix_injection_and_duplicate_targets() {
        let root = tempfile::tempdir().unwrap();
        let shim = root.path().join("codex.cmd");
        let standard =
            "@\"%dp0%\\node.exe\" \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*";
        assert!(parse_standard_npm_line(&shim, &format!("{standard} & calc.exe")).is_none());
        let targets = format!("{standard}\n{standard}\n")
            .lines()
            .filter_map(|line| parse_standard_npm_line(&shim, line))
            .collect::<Vec<_>>();
        assert_eq!(
            targets.len(),
            2,
            "duplicate targets are detected as ambiguous by npm_candidate"
        );
    }

    #[test]
    fn missing_validation_returns_structured_sanitized_diagnostics() {
        let spec = CodexLaunchSpec {
            executable_path: PathBuf::from("definitely-missing-codex"),
            argv_prefix: vec!["fixed".into()],
        };
        let error = validate_codex_runtime(&spec).unwrap_err();
        assert_eq!(error.kind, AccountConnectorErrorKind::Spawn);
        assert_eq!(error.stage, "validate_runtime");
        assert_eq!(error.launch.argv_prefix, vec!["fixed"]);
        assert!(!error
            .public_message
            .to_ascii_lowercase()
            .contains("bearer "));
    }

    #[cfg(windows)]
    #[test]
    fn windows_environment_lookup_accepts_mixed_case_path_name() {
        let environment = HashMap::from([(
            OsString::from("Path"),
            OsString::from(r"C:\fake runtime with spaces"),
        )]);
        assert_eq!(
            environment_value(&environment, "PATH").map(OsString::as_os_str),
            Some(OsStr::new(r"C:\fake runtime with spaces"))
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn selection_skips_stale_and_failed_candidates() {
        let root = tempfile::tempdir().unwrap();
        let valid = root.path().join("codex");
        executable(&valid);
        let configured = CodexLaunchSpec {
            executable_path: root.path().join("stale"),
            argv_prefix: Vec::new(),
        };
        let context = RuntimeDiscoveryContext::isolated(&[], vec![root.path().to_path_buf()]);
        let candidates = discover_codex_runtimes_with(
            &CodexRuntimeSettings {
                configured_runtime: Some(configured),
            },
            &context,
        );
        assert_eq!(
            select_codex_runtime(&candidates),
            Some(&candidates[1].launch)
        );
    }
}
