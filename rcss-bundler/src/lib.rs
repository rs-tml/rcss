#[cfg(feature = "full")]
pub mod full;
use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

#[cfg(feature = "full")]
pub use full::*;

pub const MANIFEST_PATH_CONFIG: &str = "rcss-bundler-root.path";

pub fn save_root_manifest_path(root_manifest: &Path) {
    let mut file: PathBuf = std::env::var("OUT_DIR")
        .expect("$OUT_DIR should exist.")
        .into();
    file.push(MANIFEST_PATH_CONFIG);
    std::fs::write(file, root_manifest.as_os_str().as_bytes())
        .expect("Failed to write root manifest path");
}

pub fn load_root_manifest_path(path_to_out: &Path) -> Option<PathBuf> {
    let file = path_to_out.join(MANIFEST_PATH_CONFIG);
    if file.exists() {
        let path = std::fs::read(file).expect("Failed to read root manifest path");
        let os_str = OsStr::from_bytes(&path);
        let path: &Path = os_str.as_ref();
        Some(path.into())
    } else {
        None
    }
}
