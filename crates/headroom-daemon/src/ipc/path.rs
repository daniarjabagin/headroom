use std::path::{Path, PathBuf};

use crate::config::app_dir;
use crate::error::DaemonError;

pub const SOCKET_ENV: &str = "HEADROOM_SOCKET";
pub const MAX_SOCKET_PATH: usize = 103;
const SOCKET_NAME: &str = "daemon.sock";

pub fn default_socket_path() -> Result<PathBuf, DaemonError> {
    let preferred = preferred_dir()?.join(SOCKET_NAME);
    let uid = rustix::process::getuid().as_raw();
    Ok(fit_socket_path(preferred, &std::env::temp_dir(), uid))
}

#[must_use]
pub fn fit_socket_path(preferred: PathBuf, temp_dir: &Path, uid: u32) -> PathBuf {
    if preferred.as_os_str().len() <= MAX_SOCKET_PATH {
        preferred
    } else {
        temp_dir.join(format!("headroom-{uid}.sock"))
    }
}

#[cfg(target_os = "linux")]
fn preferred_dir() -> Result<PathBuf, DaemonError> {
    match dirs::runtime_dir() {
        Some(runtime) => Ok(runtime.join("headroom")),
        None => app_dir(),
    }
}

#[cfg(not(target_os = "linux"))]
fn preferred_dir() -> Result<PathBuf, DaemonError> {
    app_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_paths_fall_back_to_the_temp_dir() {
        let temp = Path::new("/var/folders/xy/T");
        let short = PathBuf::from("/Users/ada/Library/Application Support/Headroom/daemon.sock");
        assert_eq!(fit_socket_path(short.clone(), temp, 501), short);
        let exact = PathBuf::from(format!("/{}", "a".repeat(MAX_SOCKET_PATH - 1)));
        assert_eq!(fit_socket_path(exact.clone(), temp, 501), exact);
        let long = PathBuf::from(format!("/{}", "a".repeat(MAX_SOCKET_PATH)));
        assert_eq!(
            fit_socket_path(long, temp, 501),
            PathBuf::from("/var/folders/xy/T/headroom-501.sock")
        );
    }
}
