use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

const WRITE_EXECUTABLE: &str = "cat > \"$1\" && chmod 755 \"$1\"";

/// Writes the script from a child process so a concurrent fork cannot make its exec hit ETXTBSY.
pub fn install_script(path: &Path, body: &str) -> io::Result<()> {
    let mut child = Command::new("/bin/sh")
        .args(["-c", WRITE_EXECUTABLE, "sh"])
        .arg(path)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(body.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "cannot install {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn installed_scripts_are_executable_and_run() {
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("hello");
        install_script(&program, "#!/bin/sh\necho \"hi $1\"\n").unwrap();
        let mode = std::fs::metadata(&program).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
        let output = Command::new(&program).arg("there").output().unwrap();
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "hi there\n");
    }

    #[test]
    fn a_missing_directory_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("absent").join("hello");
        assert!(install_script(&program, "#!/bin/sh\n").is_err());
    }
}
