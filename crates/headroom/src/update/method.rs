use std::path::{Path, PathBuf};

use headroom_daemon::update::{Install, Packager};
use serde::Deserialize;

const RECEIPT: &str = "headroom/install.json";
const PACKAGE_MARKER: &str = "/usr/share/headroom/installed-by-package";
const PACKAGED_PREFIX: &str = "/usr";
const DELETED_SUFFIX: &str = " (deleted)";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Receipt {
    pub method: ReceiptMethod,
    #[serde(default)]
    pub options: Vec<String>,
    pub prefix: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptMethod {
    Script,
    Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub exe: PathBuf,
    pub receipt: Option<Receipt>,
    pub marker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub install: Install,
    pub receipt: Option<Receipt>,
}

impl Receipt {
    fn binary(&self) -> PathBuf {
        self.prefix.join("bin").join("headroom")
    }
}

#[must_use]
pub fn classify(probe: &Probe) -> Install {
    if probe.exe.starts_with(PACKAGED_PREFIX) {
        let packager = probe
            .marker
            .as_deref()
            .map_or(Packager::Other, Packager::from_marker);
        return Install::Package(packager);
    }
    match &probe.receipt {
        Some(receipt)
            if receipt.method == ReceiptMethod::Script && probe.exe == receipt.binary() =>
        {
            Install::Script
        }
        _ => Install::Unknown,
    }
}

#[must_use]
pub fn detect() -> Detected {
    if !cfg!(target_os = "linux") {
        return Detected {
            install: Install::Unknown,
            receipt: None,
        };
    }
    let Ok(exe) = std::env::current_exe() else {
        return Detected {
            install: Install::Unknown,
            receipt: None,
        };
    };
    let receipt = dirs::data_dir().map(|data| data.join(RECEIPT));
    detect_at(&exe, receipt.as_deref(), Path::new(PACKAGE_MARKER))
}

#[must_use]
pub fn detect_at(exe: &Path, receipt: Option<&Path>, marker: &Path) -> Detected {
    let receipt = receipt.and_then(read_receipt).map(|mut receipt| {
        receipt.prefix = canonical(&receipt.prefix);
        receipt
    });
    let probe = Probe {
        exe: canonical(&live_path(exe)),
        receipt,
        marker: std::fs::read_to_string(marker).ok(),
    };
    Detected {
        install: classify(&probe),
        receipt: probe.receipt,
    }
}

fn read_receipt(path: &Path) -> Option<Receipt> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(receipt) => Some(receipt),
        Err(error) => {
            tracing::debug!(%error, path = %path.display(), "ignoring an unreadable install receipt");
            None
        }
    }
}

fn live_path(exe: &Path) -> PathBuf {
    let text = exe.to_string_lossy();
    match text.strip_suffix(DELETED_SUFFIX) {
        Some(live) => PathBuf::from(live),
        None => exe.to_path_buf(),
    }
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
#[path = "method_tests.rs"]
mod tests;
