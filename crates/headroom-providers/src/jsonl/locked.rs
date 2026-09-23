use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub(crate) struct Locked {
    path: PathBuf,
    mode: u32,
}

impl Locked {
    pub(crate) fn new(path: &Path) -> Locked {
        let mode = fs::metadata(path).unwrap().permissions().mode();
        fs::set_permissions(path, Permissions::from_mode(0o000)).unwrap();
        Locked {
            path: path.to_path_buf(),
            mode,
        }
    }

    pub(crate) fn is_enforced(&self) -> bool {
        let readable = if self.path.is_dir() {
            fs::read_dir(&self.path).is_ok()
        } else {
            fs::File::open(&self.path).is_ok()
        };
        !readable
    }
}

impl Drop for Locked {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.path, Permissions::from_mode(self.mode));
    }
}
