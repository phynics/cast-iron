use std::fmt;
use std::path::{Path, PathBuf};

/// A corpus read error. `line` is 1-based and always names the file it came
/// from, so a report can be pasted into an editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub path: PathBuf,
    pub line: usize,
    pub message: String,
}

impl Error {
    pub fn new(path: impl AsRef<Path>, line: usize, message: impl Into<String>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            line: line.max(1),
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.path.display(), self.line, self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;
