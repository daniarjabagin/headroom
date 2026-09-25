use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use gtk::glib;

const FOLDER: &str = "Headroom";

#[must_use]
pub fn share_dir() -> PathBuf {
    glib::user_special_dir(glib::UserDirectory::Pictures)
        .unwrap_or_else(|| glib::home_dir().join("Pictures"))
        .join(FOLDER)
}

pub fn write_atomically(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let target = dir.join(name);
    let temporary = dir.join(format!(".{name}.part"));
    let written = File::create(&temporary).and_then(|mut file| {
        file.write_all(bytes)?;
        file.sync_all()
    });
    if let Err(error) = written.and_then(|()| fs::rename(&temporary, &target)) {
        if let Err(cleanup) = fs::remove_file(&temporary) {
            tracing::debug!(%cleanup, "no partial share image to remove");
        }
        return Err(error);
    }
    Ok(target)
}

#[must_use]
pub fn folder_label(dir: &Path, home: &Path) -> String {
    dir.strip_prefix(home).unwrap_or(dir).display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("headroom-share-{name}-{}", std::process::id()))
    }

    #[test]
    fn writes_the_file_in_place_of_a_temporary() {
        let dir = scratch("write");
        let path = write_atomically(&dir, "card.png", b"png").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"png");
        assert!(!dir.join(".card.png.part").exists());
        let again = write_atomically(&dir, "card.png", b"new").unwrap();
        assert_eq!(fs::read(again).unwrap(), b"new");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn labels_are_relative_to_home() {
        let home = Path::new("/home/ada");
        assert_eq!(
            folder_label(Path::new("/home/ada/Pictures/Headroom"), home),
            "Pictures/Headroom"
        );
        assert_eq!(folder_label(Path::new("/srv/x"), home), "/srv/x");
    }
}
