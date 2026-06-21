use codex_utils_absolute_path::AbsolutePathBuf;
use dirs::home_dir;
use std::path::PathBuf;

/// Returns the path to the CodeForge configuration directory, which can be
/// specified by the `CODEFORGE_HOME` environment variable for the local
/// codeforge wrapper. If it is not set, defaults to `~/.codeforge`.
///
/// - If `CODEFORGE_HOME` is set, the value must exist and be a directory. The
///   value will be canonicalized and this function will Err otherwise.
/// - If `CODEFORGE_HOME` is not set, this function does not verify that the
///   default directory exists.
pub fn find_codex_home() -> std::io::Result<AbsolutePathBuf> {
    let codeforge_home_env = std::env::var("CODEFORGE_HOME")
        .ok()
        .filter(|val| !val.is_empty());
    find_codex_home_from_env(codeforge_home_env.as_deref())
}

fn find_codex_home_from_env(codeforge_home_env: Option<&str>) -> std::io::Result<AbsolutePathBuf> {
    match codeforge_home_env {
        Some(val) => validate_config_home("CODEFORGE_HOME", val),
        None => {
            let mut p = home_dir().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not find home directory",
                )
            })?;
            p.push(".codeforge");
            AbsolutePathBuf::from_absolute_path(p)
        }
    }
}

fn validate_config_home(env_name: &str, val: &str) -> std::io::Result<AbsolutePathBuf> {
    let path = PathBuf::from(val);
    let metadata = std::fs::metadata(&path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{env_name} points to {val:?}, but that path does not exist"),
        ),
        _ => std::io::Error::new(
            err.kind(),
            format!("failed to read {env_name} {val:?}: {err}"),
        ),
    })?;

    if !metadata.is_dir() {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{env_name} points to {val:?}, but that path is not a directory"),
        ))
    } else {
        let canonical = path.canonicalize().map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!("failed to canonicalize {env_name} {val:?}: {err}"),
            )
        })?;
        AbsolutePathBuf::from_absolute_path(canonical)
    }
}

#[cfg(test)]
mod tests {
    use super::find_codex_home_from_env;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use dirs::home_dir;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::io::ErrorKind;
    use tempfile::TempDir;


    #[test]
    fn find_codex_home_env_missing_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let missing = temp_home.path().join("missing-codeforge-home");
        let missing_str = missing
            .to_str()
            .expect("missing codeforge home path should be valid utf-8");

        let err = find_codex_home_from_env(Some(missing_str)).expect_err("missing CODEFORGE_HOME");
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert!(
            err.to_string().contains("CODEFORGE_HOME"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_codex_home_env_file_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let file_path = temp_home.path().join("codeforge-home.txt");
        fs::write(&file_path, "not a directory").expect("write temp file");
        let file_str = file_path
            .to_str()
            .expect("file codeforge home path should be valid utf-8");

        let err = find_codex_home_from_env(Some(file_str)).expect_err("file CODEFORGE_HOME");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("CODEFORGE_HOME"),
            "unexpected error: {err}"
        );
    }
    #[test]
    #[test]
    fn find_codex_home_uses_codeforge_home() {
        let temp_home = TempDir::new().expect("temp home");
        let temp_str = temp_home
            .path()
            .to_str()
            .expect("temp codeforge home path should be valid utf-8");

        let resolved = find_codex_home_from_env(Some(temp_str)).expect("valid CODEFORGE_HOME");
        let expected = temp_home
            .path()
            .canonicalize()
            .expect("canonicalize temp home");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn find_codex_home_ignores_codex_home() {
        let codex_home = TempDir::new().expect("codex home");
        let codex_str = codex_home.path().to_str().expect("utf-8 temp path");
        let _codex_home_guard = EnvVarGuard::set("CODEX_HOME", codex_str);

        let resolved = find_codex_home_from_env(None).expect("default CODEFORGE_HOME");
        let mut expected = home_dir().expect("home dir");
        expected.push(".codeforge");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn find_codex_home_codeforge_home_file_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let file_path = temp_home.path().join("codeforge-home.txt");
        fs::write(&file_path, "not a directory").expect("write temp file");
        let file_str = file_path
            .to_str()
            .expect("file codeforge home path should be valid utf-8");

        let err = find_codex_home_from_env(Some(file_str)).expect_err("file CODEFORGE_HOME");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("CODEFORGE_HOME"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_codex_home_without_env_uses_default_home_dir() {
        let resolved = find_codex_home_from_env(None).expect("default CODEFORGE_HOME");
        let mut expected = home_dir().expect("home dir");
        expected.push(".codeforge");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }

    struct EnvVarGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }
}
