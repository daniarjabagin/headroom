#[cfg(test)]
pub(crate) mod locked;
mod read;
mod walk;

use std::io;
use std::path::{Path, PathBuf};

use headroom_core::provider::ProviderError;

pub use read::{prune_missing, read_new_lines, read_new_lines_or_skip};
pub use walk::jsonl_files;

#[derive(Debug, thiserror::Error)]
pub enum JsonlError {
    #[error("cannot read {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

impl From<JsonlError> for ProviderError {
    fn from(error: JsonlError) -> ProviderError {
        ProviderError::LocalData(error.to_string())
    }
}

fn io_error(path: &Path) -> impl Fn(io::Error) -> JsonlError + '_ {
    move |source| JsonlError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn warn_skipped(path: &Path, error: &io::Error) {
    if error.kind() != io::ErrorKind::NotFound {
        tracing::warn!(path = %path.display(), kind = %error.kind(), "skipping unreadable log path");
    }
}
