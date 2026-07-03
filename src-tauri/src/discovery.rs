use std::path::PathBuf;

pub(crate) fn default_app_data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("TOKENSTACK_APP_DATA_DIR") {
        return PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(path).join("tokenstack");
    }
    if let Some(path) = std::env::var_os("APPDATA") {
        return PathBuf::from(path).join("TokenStack");
    }
    default_user_home()
        .join(".local")
        .join("share")
        .join("tokenstack")
}

pub(crate) fn default_local_history_roots() -> Vec<PathBuf> {
    if let Some(paths) = std::env::var_os("TOKENSTACK_LOCAL_HISTORY_ROOTS") {
        return std::env::split_paths(&paths).collect();
    }
    let mut roots = Vec::new();
    for home in default_user_homes() {
        push_codex_history_roots(&mut roots, home.join(".codex"));
    }
    for key in ["APPDATA", "LOCALAPPDATA"] {
        if let Some(path) = std::env::var_os(key) {
            push_codex_history_roots(&mut roots, PathBuf::from(path).join("codex"));
        }
    }
    roots
}

pub(crate) fn default_auth_home() -> PathBuf {
    default_user_home()
}

fn default_user_home() -> PathBuf {
    default_user_homes()
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("."))
}

fn default_user_homes() -> Vec<PathBuf> {
    let mut homes = Vec::new();
    for key in ["HOME", "USERPROFILE"] {
        if let Some(path) = std::env::var_os(key) {
            push_unique(&mut homes, PathBuf::from(path));
        }
    }
    if homes.is_empty() {
        homes.push(PathBuf::from("."));
    }
    homes
}

fn push_codex_history_roots(roots: &mut Vec<PathBuf>, base: PathBuf) {
    for child in ["sessions", "history", "archive", "archived_sessions"] {
        push_unique(roots, base.join(child));
    }
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    struct EnvRestore {
        values: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl EnvRestore {
        fn capture(names: &[&'static str]) -> Self {
            Self {
                values: names
                    .iter()
                    .map(|name| (*name, std::env::var_os(name)))
                    .collect(),
            }
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (name, value) in &self.values {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }

    #[test]
    fn local_history_roots_use_userprofile_when_home_is_missing() {
        let _lock = env_lock();
        let _restore =
            EnvRestore::capture(&["HOME", "USERPROFILE", "TOKENSTACK_LOCAL_HISTORY_ROOTS"]);
        let profile = tempdir().unwrap();

        std::env::remove_var("HOME");
        std::env::remove_var("TOKENSTACK_LOCAL_HISTORY_ROOTS");
        std::env::set_var("USERPROFILE", profile.path());

        let roots = default_local_history_roots();

        assert!(roots.contains(&profile.path().join(".codex").join("sessions")));
        assert!(roots.contains(&profile.path().join(".codex").join("history")));
        assert!(roots.contains(&profile.path().join(".codex").join("archive")));
    }

    #[test]
    fn local_history_roots_include_current_codex_archived_sessions_directory() {
        let _lock = env_lock();
        let _restore =
            EnvRestore::capture(&["HOME", "USERPROFILE", "TOKENSTACK_LOCAL_HISTORY_ROOTS"]);
        let profile = tempdir().unwrap();

        std::env::remove_var("HOME");
        std::env::remove_var("TOKENSTACK_LOCAL_HISTORY_ROOTS");
        std::env::set_var("USERPROFILE", profile.path());

        let roots = default_local_history_roots();

        assert!(roots.contains(&profile.path().join(".codex").join("archived_sessions")));
    }

    #[test]
    fn local_history_roots_include_appdata_codex_locations() {
        let _lock = env_lock();
        let _restore = EnvRestore::capture(&[
            "HOME",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "TOKENSTACK_LOCAL_HISTORY_ROOTS",
        ]);
        let profile = tempdir().unwrap();
        let appdata = tempdir().unwrap();
        let local_appdata = tempdir().unwrap();

        std::env::remove_var("HOME");
        std::env::remove_var("TOKENSTACK_LOCAL_HISTORY_ROOTS");
        std::env::set_var("USERPROFILE", profile.path());
        std::env::set_var("APPDATA", appdata.path());
        std::env::set_var("LOCALAPPDATA", local_appdata.path());

        let roots = default_local_history_roots();

        assert!(roots.contains(&appdata.path().join("codex").join("sessions")));
        assert!(roots.contains(&appdata.path().join("codex").join("history")));
        assert!(roots.contains(&appdata.path().join("codex").join("archive")));
        assert!(roots.contains(&local_appdata.path().join("codex").join("sessions")));
        assert!(roots.contains(&local_appdata.path().join("codex").join("history")));
        assert!(roots.contains(&local_appdata.path().join("codex").join("archive")));
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }
}
