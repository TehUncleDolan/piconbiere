//! Filesystem helpers.

use eyre::{Result, WrapErr};
use once_cell::unsync::Lazy;
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Clean a name to safely use it as directory name.
pub fn sanitize_name(name: &str) -> PathBuf {
    // Linux only is not that restrictive, but Windows is another story...
    // See https://docs.microsoft.com/en-us/windows/win32/fileio/naming-a-file
    let dir_illegal_chars = Lazy::new(|| {
        Regex::new(r#"[/\?<>\\:\*\|"]"#).expect("invalid chars regexp")
    });
    let dir_illegal_trailing =
        Lazy::new(|| Regex::new(r#"[\. ]+$"#).expect("invalid trailing regex"));

    let name = dir_illegal_trailing.replace(name, "");

    dir_illegal_chars
        .replace_all(&name, "_")
        .into_owned()
        .into()
}

/// Recursively create a directory and all of its parent if necessary.
pub fn mkdir_p(path: &Path) -> Result<()> {
    fs::create_dir_all(&path)
        .with_context(|| format!("mkdir_p {}", path.display()))
}

/// Write a file atomically (using a tempfile + atomic rename).
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let mut tmp_path = path.to_path_buf();
    tmp_path.set_extension("part");

    fs::write(&tmp_path, data)
        .with_context(|| format!("write {}", tmp_path.display()))?;

    fs::rename(&tmp_path, path)
        .with_context(|| format!("rename to {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_trailing() {
        let expected: PathBuf = "foo".into();

        assert_eq!(sanitize_name("foo   "), expected);
        assert_eq!(sanitize_name("foo."), expected);
        assert_eq!(sanitize_name("foo. ."), expected);
        assert_eq!(sanitize_name("foo. . "), expected);
    }

    #[test]
    fn test_sanitize_invalid() {
        let expected: PathBuf = "foo_bar".into();

        assert_eq!(sanitize_name("foo/bar/"), PathBuf::from("foo_bar_"));
        assert_eq!(sanitize_name("foo:bar"), expected);
        assert_eq!(sanitize_name("foo?bar"), expected);
        assert_eq!(sanitize_name("foo|bar"), expected);
        assert_eq!(sanitize_name("foo*bar"), expected);
        assert_eq!(sanitize_name("foo>bar"), expected);
        assert_eq!(sanitize_name("foo<bar"), expected);
        assert_eq!(sanitize_name("foo\\bar"), expected);
        assert_eq!(sanitize_name("foo\"bar"), expected);
    }
}
