use std::io::Write;
use std::path::Path;
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use headroom_daemon::update::{GithubRelease, Version};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::feed::GithubFeed;
use super::report::Reporter;
use super::verify::verify;

const SUMS: &str = "SHA256SUMS";
const INSTALLER: &str = "install.sh";

pub struct Bundle<'a> {
    pub release: &'a GithubRelease,
    pub version: &'a Version,
    pub arch: &'a str,
}

impl Bundle<'_> {
    fn name(&self) -> String {
        format!("headroom-{}-{}-linux-musl", self.version, self.arch)
    }

    fn asset_url(&self, name: &str) -> Result<&str> {
        match self.release.asset(name) {
            Some(asset) => Ok(&asset.browser_download_url),
            None => bail!("release {} has no {name}", self.version),
        }
    }
}

pub async fn install<W: Write>(
    feed: &GithubFeed,
    bundle: &Bundle<'_>,
    options: &[String],
    reporter: &mut Reporter<W>,
) -> Result<()> {
    let name = bundle.name();
    let archive_name = format!("{name}.tar.gz");
    let sums_url = bundle.asset_url(SUMS)?;
    let archive_url = bundle.asset_url(&archive_name)?;
    reporter.step(&format!("Downloading Headroom {}…", bundle.version))?;
    let sums = feed.download(sums_url).await?;
    let archive = feed.download(archive_url).await?;
    reporter.step("Verifying the checksum…")?;
    let sums = String::from_utf8(sums).context("SHA256SUMS is not text")?;
    verify(&sums, &archive_name, &archive)?;
    let work = tempfile::tempdir().context("could not create a temporary directory")?;
    reporter.step("Unpacking…")?;
    unpack(work.path(), &archive_name, &archive).await?;
    run_installer(&work.path().join(name).join(INSTALLER), options, reporter).await
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

async fn run_installer<W: Write>(
    script: &Path,
    options: &[String],
    reporter: &mut Reporter<W>,
) -> Result<()> {
    if !script.is_file() {
        bail!("the release bundle has no {INSTALLER}");
    }
    let mut child = Command::new("bash")
        .arg(script)
        .args(options)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .context("could not run install.sh")?;
    if let Some(stdout) = child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        while let Some(line) = lines.next_line().await? {
            reporter.installer_line(&line)?;
        }
    }
    let status = child.wait().await.context("install.sh did not finish")?;
    if !status.success() {
        bail!("install.sh failed ({status})");
    }
    Ok(())
}
