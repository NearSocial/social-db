use std::fs;
use std::path::{Path, PathBuf};

/// Get the canonical workspace directory.
/// From https://stackoverflow.com/a/74942075
pub fn get_workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    fs::canonicalize(cargo_path.parent().unwrap().to_path_buf()).expect("can canonicalize workspace dir")
}


