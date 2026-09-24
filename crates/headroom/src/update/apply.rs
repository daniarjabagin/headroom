use std::io::Write;
use std::path::Path;
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use headroom_daemon::update::{GithubRelease, Version};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout, Command};

use super::feed::GithubFeed;
use super::method::TrayVariant;
use super::report::Reporter;
use super::signature::{PublicKey, verify_signature};
use super::verify::verify;

const SUMS: &str = "SHA256SUMS";
const SIGNATURE: &str = "SHA256SUMS.sig";
const SUMS_LIMIT: usize = 64 * 1024;
const SIGNATURE_LIMIT: usize = 1024;
const ARCHIVE_LIMIT: usize = 64 * 1024 * 1024;
const INSTALLER: &str = "install.sh";
const TRAY_DIR: &str = "tray";
const CHUNK_BYTES: usize = 4_096;

pub struct Bundle<'a> {
    pub release: &'a GithubRelease,
    pub version: &'a Version,
    pub arch: &'a str,
    pub release_key: &'a PublicKey,
    pub tray: Option<TrayVariant>,
}

struct Download<'a> {
    name: String,
    url: &'a str,
}

impl Bundle<'_> {
    fn name(&self) -> String {
        format!("headroom-{}-{}-linux-musl", self.version, self.arch)
    }

    fn tray_name(&self, variant: TrayVariant) -> String {
        format!(
            "headroom-tray-{}-{}-{}",
            self.version,
            self.arch,
            variant.suffix()
        )
    }

    fn archive(&self, name: String) -> Result<Download<'_>> {
        let url = self.asset_url(&format!("{name}.tar.gz"))?;
        Ok(Download { name, url })
    }

    fn tray_archive(&self, variant: TrayVariant) -> Result<Download<'_>> {
        let name = self.tray_name(variant);
        match self.release.asset(&format!("{name}.tar.gz")) {
            Some(asset) => Ok(Download {
                url: &asset.browser_download_url,
                name,
            }),
            None => bail!(
                "release {} has no {name}.tar.gz, so the installed Headroom tray cannot be updated",
                self.version
            ),
        }
    }

    fn asset_url(&self, name: &str) -> Result<&str> {
        match self.release.asset(name) {
            Some(asset) => Ok(&asset.browser_download_url),
            None => bail!("release {} has no {name}", self.version),
        }
    }
}

pub async fn install<W: Write, E: Write>(
    feed: &GithubFeed,
    bundle: &Bundle<'_>,
    options: &[String],
    reporter: &mut Reporter<W, E>,
) -> Result<()> {
    let sums_url = bundle.asset_url(SUMS)?;
    let signature_url = bundle.asset_url(SIGNATURE)?;
    let main = bundle.archive(bundle.name())?;
    let tray = match bundle.tray {
        Some(variant) => Some(bundle.tray_archive(variant)?),
        None => None,
    };
    reporter.step(&format!("Downloading Headroom {}…", bundle.version))?;
    let sums = feed.download(sums_url, SUMS_LIMIT).await?;
    let signature = feed.download(signature_url, SIGNATURE_LIMIT).await?;
    reporter.step("Verifying the signature…")?;
    verify_signature(bundle.release_key, &sums, &signature)?;
    let sums = String::from_utf8(sums).context("SHA256SUMS is not text")?;
    let work = tempfile::tempdir().context("could not create a temporary directory")?;
    fetch(feed, &sums, &main, work.path(), reporter).await?;
    let root = work.path().join(&main.name);
    if let Some(tray) = &tray {
        fetch(feed, &sums, tray, work.path(), reporter).await?;
        place_tray(&work.path().join(&tray.name), &root.join(TRAY_DIR)).await?;
    }
    run_installer(&root.join(INSTALLER), options, reporter).await
}

async fn fetch<W: Write, E: Write>(
    feed: &GithubFeed,
    sums: &str,
    download: &Download<'_>,
    dir: &Path,
    reporter: &mut Reporter<W, E>,
) -> Result<()> {
    let archive_name = format!("{}.tar.gz", download.name);
    let archive = feed.download(download.url, ARCHIVE_LIMIT).await?;
    reporter.step("Verifying the checksum…")?;
    verify(sums, &archive_name, &archive)?;
    reporter.step("Unpacking…")?;
    unpack(dir, &archive_name, &archive).await
}

async fn place_tray(unpacked: &Path, target: &Path) -> Result<()> {
    if !unpacked.is_dir() {
        bail!("the tray archive has no {} directory", unpacked.display());
    }
    tokio::fs::rename(unpacked, target)
        .await
        .with_context(|| format!("could not move the tray bundle to {}", target.display()))
}

async fn unpack(dir: &Path, archive_name: &str, archive: &[u8]) -> Result<()> {
    let path = dir.join(archive_name);
    tokio::fs::write(&path, archive)
        .await
        .with_context(|| format!("could not write {}", path.display()))?;
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&path)
        .arg("-C")
        .arg(dir)
        .stdin(Stdio::null())
        .status()
        .await
        .context("could not run tar")?;
    if !status.success() {
        bail!("could not unpack {archive_name} (tar {status})");
    }
    Ok(())
}

async fn run_installer<W: Write, E: Write>(
    script: &Path,
    options: &[String],
    reporter: &mut Reporter<W, E>,
) -> Result<()> {
    if !script.is_file() {
        bail!("the release bundle has no {INSTALLER}");
    }
    let mut child = Command::new("bash")
        .arg(script)
        .args(options)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("could not run install.sh")?;
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        bail!("could not read the output of install.sh");
    };
    relay(stdout, stderr, reporter).await?;
    let status = child.wait().await.context("install.sh did not finish")?;
    if !status.success() {
        bail!("install.sh failed ({status})");
    }
    Ok(())
}

async fn relay<W: Write, E: Write>(
    stdout: ChildStdout,
    mut stderr: ChildStderr,
    reporter: &mut Reporter<W, E>,
) -> Result<()> {
    let mut lines = BufReader::new(stdout).lines();
    let mut chunk = [0_u8; CHUNK_BYTES];
    let (mut stdout_open, mut stderr_open) = (true, true);
    while stdout_open || stderr_open {
        tokio::select! {
            line = lines.next_line(), if stdout_open => match line? {
                Some(line) => reporter.installer_line(&line)?,
                None => stdout_open = false,
            },
            read = stderr.read(&mut chunk), if stderr_open => match read? {
                0 => stderr_open = false,
                count => reporter.installer_diagnostics(&chunk[..count])?,
            },
        }
    }
    Ok(())
}
