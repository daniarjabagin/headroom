use std::fs::{self, DirBuilder, File, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use tracing_subscriber::fmt::MakeWriter;

pub const LOG_CAP_BYTES: u64 = 2 * 1024 * 1024;
const FILE_MODE: u32 = 0o600;
const DIR_MODE: u32 = 0o700;

pub struct RotatingFile {
    path: PathBuf,
    cap: u64,
    open: Mutex<OpenFile>,
}

struct OpenFile {
    file: File,
    len: u64,
}

pub struct Record<'a>(&'a RotatingFile);

impl RotatingFile {
    pub fn open(path: &Path, cap: u64) -> io::Result<RotatingFile> {
        if let Some(dir) = path.parent() {
            private_dir(dir)?;
        }
        Ok(RotatingFile {
            path: path.to_owned(),
            cap,
            open: Mutex::new(open_private(path)?),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn write_record(&self, bytes: &[u8]) -> io::Result<()> {
        let mut open = self.lock();
        let incoming = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if open.len > 0 && open.len.saturating_add(incoming) > self.cap {
            *open = self.rotate()?;
        }
        open.file.write_all(bytes)?;
        open.len = open.len.saturating_add(incoming);
        Ok(())
    }

    fn rotate(&self) -> io::Result<OpenFile> {
        fs::rename(&self.path, rotated_path(&self.path))?;
        open_private(&self.path)
    }

    fn lock(&self) -> MutexGuard<'_, OpenFile> {
        self.open.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<'a> MakeWriter<'a> for RotatingFile {
    type Writer = Record<'a>;

    fn make_writer(&'a self) -> Record<'a> {
        Record(self)
    }
}

impl Write for Record<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write_record(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn rotated_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".1");
    PathBuf::from(name)
}

fn private_dir(dir: &Path) -> io::Result<()> {
    DirBuilder::new()
        .recursive(true)
        .mode(DIR_MODE)
        .create(dir)?;
    fs::set_permissions(dir, Permissions::from_mode(DIR_MODE))
}

fn open_private(path: &Path) -> io::Result<OpenFile> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(FILE_MODE)
        .open(path)?;
    file.set_permissions(Permissions::from_mode(FILE_MODE))?;
    let len = file.metadata()?.len();
    Ok(OpenFile { file, len })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    fn write(sink: &RotatingFile, text: &str) {
        sink.make_writer().write_all(text.as_bytes()).unwrap();
    }

    #[test]
    fn the_file_and_its_directory_are_private() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state/headroom/headroom.log");
        let sink = RotatingFile::open(&path, 64).unwrap();
        write(&sink, "started\n");
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(path.parent().unwrap()), 0o700);
        assert_eq!(fs::read_to_string(&path).unwrap(), "started\n");
    }

    #[test]
    fn existing_files_are_tightened_and_appended() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("headroom.log");
        fs::write(&path, "old\n").unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o644)).unwrap();
        fs::set_permissions(temp.path(), Permissions::from_mode(0o755)).unwrap();
        let sink = RotatingFile::open(&path, 64).unwrap();
        write(&sink, "new\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), "old\nnew\n");
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(temp.path()), 0o700);
    }

    #[test]
    fn a_record_that_would_pass_the_cap_starts_a_new_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("headroom.log");
        let sink = RotatingFile::open(&path, 10).unwrap();
        write(&sink, "12345\n");
        write(&sink, "abc\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), "12345\nabc\n");
        write(&sink, "xyz\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), "xyz\n");
        assert_eq!(
            fs::read_to_string(rotated_path(&path)).unwrap(),
            "12345\nabc\n"
        );
        assert_eq!(mode(&path), 0o600);
        write(&sink, "second\n");
        write(&sink, "third\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), "third\n");
        assert_eq!(fs::read_to_string(rotated_path(&path)).unwrap(), "second\n");
    }

    #[test]
    fn a_full_file_from_an_earlier_run_is_rotated_on_the_first_record() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("headroom.log");
        fs::write(&path, "0123456789").unwrap();
        let sink = RotatingFile::open(&path, 10).unwrap();
        write(&sink, "fresh\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), "fresh\n");
        assert_eq!(
            fs::read_to_string(rotated_path(&path)).unwrap(),
            "0123456789"
        );
    }

    #[test]
    fn a_single_record_larger_than_the_cap_is_still_written() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("headroom.log");
        let sink = RotatingFile::open(&path, 4).unwrap();
        write(&sink, "longer than four\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), "longer than four\n");
    }

    #[test]
    fn the_production_cap_is_two_mebibytes() {
        assert_eq!(LOG_CAP_BYTES, 2_097_152);
    }
}
