use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::CString,
    fs,
    path::Path,
};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

use tempfile::TempDir;

use crate::error::AppError;

#[derive(Clone, Debug, Default)]
pub struct Artifacts {
    pub feed_files: BTreeMap<String, Vec<u8>>,
    pub headers_bytes: Vec<u8>,
    pub index_bytes: Vec<u8>,
    pub state_bytes: Vec<u8>,
}

pub fn has_persisted_changes(
    state_path: &Path,
    output_dir: &Path,
    artifacts: &Artifacts,
) -> Result<bool, AppError> {
    if read_optional(state_path)? != Some(artifacts.state_bytes.clone()) {
        return Ok(true);
    }
    if read_optional(&output_dir.join("_headers"))? != Some(artifacts.headers_bytes.clone()) {
        return Ok(true);
    }
    if !output_dir.join("index.html").is_file() {
        return Ok(true);
    }
    for (relative, bytes) in &artifacts.feed_files {
        if read_optional(&output_dir.join(relative))? != Some(bytes.clone()) {
            return Ok(true);
        }
    }
    Ok(existing_paths(output_dir)? != expected_paths(artifacts))
}

pub fn persist(
    state_path: &Path,
    output_dir: &Path,
    artifacts: &Artifacts,
) -> Result<(), AppError> {
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
    }
    fs::write(state_path, &artifacts.state_bytes)
        .map_err(|error| AppError::io(state_path, error))?;
    let parent = output_dir.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
    let temp = TempDir::new_in(parent).map_err(|error| AppError::io(parent, error))?;
    let stage_dir = temp.path().join("dist");
    fs::create_dir_all(stage_dir.join("feeds/article"))
        .map_err(|error| AppError::io(&stage_dir, error))?;
    fs::create_dir_all(stage_dir.join("feeds/comments"))
        .map_err(|error| AppError::io(&stage_dir, error))?;
    fs::write(stage_dir.join("_headers"), &artifacts.headers_bytes)
        .map_err(|error| AppError::io(stage_dir.join("_headers"), error))?;
    fs::write(stage_dir.join("index.html"), &artifacts.index_bytes)
        .map_err(|error| AppError::io(stage_dir.join("index.html"), error))?;
    for (relative, bytes) in &artifacts.feed_files {
        fs::write(stage_dir.join(relative), bytes)
            .map_err(|error| AppError::io(stage_dir.join(relative), error))?;
    }
    replace_output_dir(output_dir, &stage_dir)
}

fn existing_paths(root: &Path) -> Result<BTreeSet<String>, AppError> {
    let mut paths = BTreeSet::new();
    if root.exists() {
        collect_paths(root, root, &mut paths)?;
    }
    Ok(paths)
}

fn expected_paths(artifacts: &Artifacts) -> BTreeSet<String> {
    let mut paths = BTreeSet::from(["_headers".to_string(), "index.html".to_string()]);
    paths.extend(artifacts.feed_files.keys().cloned());
    paths
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, AppError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AppError::io(path, error)),
    }
}

fn collect_paths(
    root: &Path,
    current: &Path,
    paths: &mut BTreeSet<String>,
) -> Result<(), AppError> {
    for entry in fs::read_dir(current).map_err(|error| AppError::io(current, error))? {
        let entry = entry.map_err(|error| AppError::io(current, error))?;
        let path = entry.path();
        if path.is_dir() {
            collect_paths(root, &path, paths)?;
        } else {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            paths.insert(relative);
        }
    }
    Ok(())
}

fn replace_output_dir(output_dir: &Path, stage_dir: &Path) -> Result<(), AppError> {
    if !output_dir.exists() {
        return fs::rename(stage_dir, output_dir).map_err(|error| AppError::io(output_dir, error));
    }
    swap_paths(output_dir, stage_dir).map_err(|error| AppError::io(output_dir, error))?;
    fs::remove_dir_all(stage_dir).map_err(|error| AppError::io(stage_dir, error))
}

#[cfg(target_os = "linux")]
fn swap_paths(left: &Path, right: &Path) -> std::io::Result<()> {
    let left = path_cstring(left)?;
    let right = path_cstring(right)?;
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            left.as_ptr(),
            libc::AT_FDCWD,
            right.as_ptr(),
            libc::RENAME_EXCHANGE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
fn swap_paths(left: &Path, right: &Path) -> std::io::Result<()> {
    let left = path_cstring(left)?;
    let right = path_cstring(right)?;
    let result = unsafe { libc::renamex_np(left.as_ptr(), right.as_ptr(), libc::RENAME_SWAP) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn swap_paths(_left: &Path, _right: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "atomic directory swap is unsupported on this platform",
    ))
}

#[cfg(unix)]
fn path_cstring(path: &Path) -> std::io::Result<CString> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains NUL"))
}
