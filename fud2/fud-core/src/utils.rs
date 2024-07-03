use camino::{Utf8Path, Utf8PathBuf};
use pathdiff::diff_utf8_paths;

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
    Utf8Path::new(path)
        .file_stem()
        .expect("Failed to get a basename from the path.")
}
