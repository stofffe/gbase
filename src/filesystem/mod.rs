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
