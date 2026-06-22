//! Static asset serving. The UI lives as plain files under `assets/` and is read
//! from disk at request time, so the UI can be edited without recompiling.
//!
//! The assets directory is resolved at startup (see [`assets_dir`]):
//!   1. `$WEBMOTE_ASSETS` if set,
//!   2. `<dir of the running binary>/assets`,
//!   3. `./assets` (current working directory) as a last resort.

use std::path::{Path, PathBuf};

/// Resolves the directory that holds the UI files.
pub fn assets_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("WEBMOTE_ASSETS") {
        return PathBuf::from(dir);
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let candidate = parent.join("assets");
        if candidate.is_dir() {
            return candidate;
        }
    }
    PathBuf::from("assets")
}

/// Reads the asset for an HTTP request path, returning its bytes and MIME type.
/// `/` maps to `index.html`. Returns `None` for traversal attempts or misses.
pub fn load(dir: &Path, request_path: &str) -> Option<(Vec<u8>, &'static str)> {
    let rel = match request_path {
        "/" | "" => "index.html",
        p => p.trim_start_matches('/'),
    };

    // Reject anything that could escape the assets directory.
    if rel.is_empty() || rel.contains("..") || rel.starts_with('/') {
        return None;
    }

    let bytes = std::fs::read(dir.join(rel)).ok()?;
    Some((bytes, content_type(rel)))
}

/// Maps a file name to a Content-Type by extension.
fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}
