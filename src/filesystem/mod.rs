mod platforms;
use std::path::{self, PathBuf};

pub use platforms::*;

pub fn normalize_path(path: impl AsRef<std::path::Path>) -> PathBuf {
    let mut out = PathBuf::new();

    for component in path.as_ref().components() {
        match component {
            path::Component::CurDir => {
                // skip "."
            }

            path::Component::ParentDir => {
                out.pop();
            }

            path::Component::Normal(part) => {
                out.push(part);
            }

            path::Component::RootDir | path::Component::Prefix(_) => {
                out.clear();
                out.push(component);
            }
        }
    }

    out
}

//
// Commands
//

pub fn write_temporary_bytes(
    ctx: &crate::Context,
    path: impl AsRef<std::path::Path>,
    data: &[u8],
) -> Result<(), WriteFileError> {
    ctx.filesystem.write_temporary_bytes(path, data)
}

pub fn load_temporary_bytes(
    ctx: &crate::Context,
    path: impl AsRef<std::path::Path>,
) -> Result<Vec<u8>, LoadFileError> {
    ctx.filesystem.load_temporary_bytes(path)
}

pub fn write_temporary_string(
    ctx: &crate::Context,
    path: impl AsRef<std::path::Path>,
    data: &str,
) -> Result<(), WriteFileError> {
    ctx.filesystem.write_temporary_string(path, data)
}

pub fn load_temporary_string(
    ctx: &crate::Context,
    path: impl AsRef<std::path::Path>,
) -> Result<String, LoadFileError> {
    ctx.filesystem.load_temporary_string(path)
}

// TODO: use filesystem context

/// Path to temporary storage folder
pub fn tmp_path() -> &'static str {
    "assets/tmp"
}

/// Load bytes from assets folder
///
/// # Input/Output
///
/// Path from \[PROJECT ROOT\]/assets/ => std::io::Result\<??\>
///
/// # Mode & Platform
///
/// Debug:      Load using `std::fs::read`
/// Release:    Load using `load_bytes!`
///
/// WASM: Always uses `load_bytes!`
///
/// # Examples
/// ```
/// use gbase::filesystem;
/// let shader_bytes = filesystem::load_b!("shaders/shader.wgsl").unwrap();
/// ```
///
#[macro_export]
macro_rules! load_b {
    ($path:literal) => {
        if cfg!(target_arch = "wasm32") || cfg!(not(debug_assertions)) {
            Ok(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path)).to_vec())
        } else {
            std::fs::read(concat!("assets/", $path))
        }
    };
}

/// Load string from assets folder
///
/// # Input/Output
///
/// Path from \[PROJECT ROOT\]/assets/ => std::io::Result\<String\>
///
/// # Mode & Platform
///
/// Debug:      Load using `std::fs::read_to_string`
/// Release:    Load using `load_str!`
///
/// WASM: Always uses `load_str!`
///
/// # Examples
/// ```
/// use gbase::filesystem;
/// let shader_str = filesystem::load_s!("shaders/shader.wgsl").unwrap();
/// ```
///
#[macro_export]
macro_rules! load_s {
    ($path:literal) => {
        if cfg!(target_arch = "wasm32") || cfg!(not(debug_assertions)) {
            Ok(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path)).to_string())
        } else {
            std::fs::read_to_string(concat!("assets/", $path))
        }
    };
}

pub use load_b;
pub use load_s;

#[cfg(test)]
mod tests {
    use crate::filesystem::normalize_path;
    use std::path::PathBuf;

    #[test]
    fn test_dot_segments() {
        assert_eq!(normalize_path("a/./b"), PathBuf::from("a/b"));
    }

    #[test]
    fn test_parent_segments() {
        assert_eq!(normalize_path("a/b/../c"), PathBuf::from("a/c"));
    }

    #[test]
    fn test_multiple_parents() {
        assert_eq!(normalize_path("a/b/c/../../d"), PathBuf::from("a/d"));
    }

    #[test]
    fn test_leading_parent() {
        // behavior choice: we allow popping beyond root -> stays minimal
        assert_eq!(normalize_path("../a"), PathBuf::from("a"));
    }

    #[test]
    fn test_complex() {
        assert_eq!(normalize_path("./a/../b/./c"), PathBuf::from("b/c"));
    }
}
