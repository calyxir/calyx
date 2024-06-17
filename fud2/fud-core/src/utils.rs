use camino::{Utf8Path, Utf8PathBuf};
use pathdiff::diff_utf8_paths;
use std::path::Path;

/// Get a version of `path` that works when the working directory is `base`. This is
/// opportunistically a relative path, but we can always fall back to an absolute path to make sure
/// the path still works.
pub fn relative_path(path: &Utf8Path, base: &Utf8Path) -> Utf8PathBuf {
    match diff_utf8_paths(path, base) {
        Some(p) => p,
        None => path
            .canonicalize_utf8()
            .expect("could not get absolute path"),
    }
}

/// Get the basename of a file as designated in a &str.
pub fn basename(path: &str) -> &str {
    let file_stem = Path::new(path).file_stem().unwrap_or_else(|| {
        panic!("Could not extract a file name from path: {}", path)
    });
    return file_stem.to_str().unwrap_or_else(|| {
        unreachable!(
            "Error when converting file_name to &str. Shouldn't be possible."
        )
    });
}
