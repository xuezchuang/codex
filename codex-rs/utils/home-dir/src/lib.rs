use codex_utils_absolute_path::AbsolutePathBuf;
use dirs::home_dir;
use std::path::PathBuf;

/// Returns the path to the xcode configuration directory, which can be
/// specified by the `XCODE_HOME` environment variable. If `XCODE_HOME` is not
/// set, defaults to `~/.xcode`.
///
/// This function **does not** read the upstream `CODEX_HOME` environment
/// variable. The xcode binary is intentionally isolated from upstream codex:
/// if you set `CODEX_HOME` for the upstream binary, it must not leak into
/// xcode, and vice versa. Run `xcode.ps1` to launch xcode with the right
/// environment; do not share `CODEX_HOME` between the two binaries.
///
/// - If `XCODE_HOME` is set, the value must exist and be a directory. The
///   value will be canonicalized and this function will Err otherwise.
/// - If `XCODE_HOME` is not set, this function does not verify that the
///   default directory exists.
pub fn find_codex_home() -> std::io::Result<AbsolutePathBuf> {
    let xcode_home_env = std::env::var("XCODE_HOME")
        .ok()
        .filter(|val| !val.is_empty());
    find_codex_home_from_env(xcode_home_env.as_deref())
}

fn find_codex_home_from_env(
    xcode_home_env: Option<&str>,
) -> std::io::Result<AbsolutePathBuf> {
    match xcode_home_env {
        Some(val) => validate_xcode_home(val),
        None => {
            // Default to `~/.xcode` rather than upstream's `~/.codex`, so the
            // two binaries never share a config directory by accident.
            let mut p = home_dir().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not find home directory",
                )
            })?;
            p.push(".xcode");
            AbsolutePathBuf::from_absolute_path(p)
        }
    }
}

fn validate_xcode_home(val: &str) -> std::io::Result<AbsolutePathBuf> {
    let path = PathBuf::from(val);
    let metadata = std::fs::metadata(&path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("XCODE_HOME points to {val:?}, but that path does not exist"),
        ),
        _ => std::io::Error::new(
            err.kind(),
            format!("failed to read XCODE_HOME {val:?}: {err}"),
        ),
    })?;

    if !metadata.is_dir() {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("XCODE_HOME points to {val:?}, but that path is not a directory"),
        ))
    } else {
        let canonical = path.canonicalize().map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!("failed to canonicalize XCODE_HOME {val:?}: {err}"),
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
    fn xcode_home_missing_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let missing = temp_home.path().join("missing-xcode-home");
        let missing_str = missing
            .to_str()
            .expect("missing xcode home path should be valid utf-8");

        let err = find_codex_home_from_env(Some(missing_str)).expect_err("missing XCODE_HOME");
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert!(
            err.to_string().contains("XCODE_HOME"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn xcode_home_file_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let file_path = temp_home.path().join("xcode-home.txt");
        fs::write(&file_path, "not a directory").expect("write temp file");
        let file_str = file_path
            .to_str()
            .expect("file xcode home path should be valid utf-8");

        let err = find_codex_home_from_env(Some(file_str)).expect_err("file XCODE_HOME");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("not a directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn xcode_home_valid_directory_canonicalizes() {
        let temp_home = TempDir::new().expect("temp home");
        let temp_str = temp_home
            .path()
            .to_str()
            .expect("temp xcode home path should be valid utf-8");

        let resolved = find_codex_home_from_env(Some(temp_str)).expect("valid XCODE_HOME");
        let expected = temp_home
            .path()
            .canonicalize()
            .expect("canonicalize temp home");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }


    #[test]
    fn xcode_home_ignores_codex_home_even_if_set() {
        // Sanity: this function must not read CODEX_HOME. We can't easily set
        // an env var from inside this process without affecting other tests,
        // so we just assert the no-env path defaults to ~/.xcode, not ~/.codex.
        let resolved = find_codex_home_from_env(None).expect("default xcode home");
        let mut expected = home_dir().expect("home dir");
        expected.push(".xcode");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }
}
