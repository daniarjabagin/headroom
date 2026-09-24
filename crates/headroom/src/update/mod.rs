mod apply;
mod feed;
mod lossy;
mod method;
mod origins;
mod report;
mod signature;
mod verify;

use std::io::{self, BufRead, IsTerminal, Write};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use headroom_daemon::update::{Install, LATEST_RELEASE_API, Release, UpdateConfig, Version};

use crate::cli::{ProgressFormat, UpdateArgs};
use apply::Bundle;
use lossy::LossyWriter;
use method::Detected;
use origins::Origins;
use report::{Finished, Reporter};
use signature::{PublicKey, RELEASE_KEY};

pub use feed::GithubFeed;

pub struct Updater<'a> {
    pub feed: &'a GithubFeed,
    pub current: &'a Version,
    pub arch: &'a str,
    pub release_key: &'a PublicKey,
    pub detected: &'a Detected,
    pub on_path: &'a dyn Fn(&str) -> bool,
}

pub fn current_version() -> Result<Version> {
    env!("CARGO_PKG_VERSION")
        .parse()
        .context("this build's version is not a semantic version")
}

pub fn daemon_config() -> Result<UpdateConfig> {
    Ok(UpdateConfig {
        feed: Arc::new(GithubFeed::new(LATEST_RELEASE_API, Origins::github())?),
        install: method::detect().install,
        current: current_version()?,
    })
}

pub async fn run(args: &UpdateArgs) -> Result<()> {
    let feed = GithubFeed::new(LATEST_RELEASE_API, Origins::github())?;
    let current = current_version()?;
    let detected = method::detect();
    let updater = Updater {
        feed: &feed,
        current: &current,
        arch: std::env::consts::ARCH,
        release_key: &RELEASE_KEY,
        detected: &detected,
        on_path: &on_path,
    };
    if args.check {
        let text = check(&updater).await?;
        writeln!(io::stdout(), "{text}")?;
        return Ok(());
    }
    match args.progress {
        None => {
            let mut reporter = Reporter::text(lossy_stdout(), lossy_stderr());
            let approve = |question: &str| Ok(args.yes || confirm(question)?);
            let outcome = update(&updater, approve, &mut reporter).await;
            reporter.finish(outcome)
        }
        Some(ProgressFormat::Json) => {
            let mut reporter = Reporter::json(lossy_stdout(), lossy_stderr());
            let approve = |_: &str| Ok(args.yes || refuse_without_yes()?);
            let outcome = update(&updater, approve, &mut reporter).await;
            reporter.finish(outcome)
        }
    }
}

fn lossy_stdout() -> LossyWriter<io::Stdout> {
    LossyWriter::new(io::stdout())
}

fn lossy_stderr() -> LossyWriter<io::Stderr> {
    LossyWriter::new(io::stderr())
}

async fn check(updater: &Updater<'_>) -> Result<String> {
    let latest = updater.feed.release().await?.stable()?;
    let current = updater.current;
    Ok(match latest.filter(|release| release.version > *current) {
        Some(release) => format!(
            "Headroom {current} is installed; {} is available ({}).\nUpdate with: {}",
            release.version,
            release.published_at.strftime("%Y-%m-%d"),
            updater.detected.install.command(&release)
        ),
        None => format!("Headroom {current} is up to date."),
    })
}

async fn update<W: Write, E: Write>(
    updater: &Updater<'_>,
    approve: impl FnOnce(&str) -> Result<bool>,
    reporter: &mut Reporter<W, E>,
) -> Result<Finished> {
    reporter.step("Checking for a new release…")?;
    let raw = updater.feed.release().await?;
    let current = updater.current;
    let Some(release) = raw.stable()?.filter(|release| release.version > *current) else {
        return Ok(Finished {
            version: current.to_string(),
            relogin: false,
            message: format!("Headroom {current} is up to date."),
        });
    };
    let options = match (&updater.detected.install, &updater.detected.receipt) {
        (Install::Script, Some(receipt)) => &receipt.options,
        (install, _) => bail!("{}", cannot_update_itself(*install, &release)),
    };
    if !approve(&format!(
        "Update Headroom {current} to {}?",
        release.version
    ))? {
        bail!("update cancelled");
    }
    let bundle = Bundle {
        release: &raw,
        version: &release.version,
        arch: updater.arch,
        release_key: updater.release_key,
    };
    apply::install(updater.feed, &bundle, options, reporter).await?;
    Ok(finished(&release.version, options, updater.on_path))
}

fn cannot_update_itself(install: Install, release: &Release) -> String {
    match install {
        Install::Package(packager) => format!(
            "Headroom was installed from a {}, so it cannot update itself. {}",
            packager.name(),
            install.command(release)
        ),
        Install::Script | Install::Unknown => format!(
            "This Headroom was not installed by install.sh, so it cannot update itself. \
             Get Headroom {} from {}",
            release.version, release.url
        ),
    }
}

fn finished(version: &Version, options: &[String], on_path: &dyn Fn(&str) -> bool) -> Finished {
    let chosen = |flag: &str| !options.iter().any(|option| option == flag);
    let gnome = chosen("--no-gnome") && on_path("gnome-shell");
    let plasma = chosen("--no-plasma") && on_path("plasmashell");
    let reload = match (gnome, plasma) {
        (true, true) => {
            " Log out and back in to reload the GNOME Shell extension and the Plasma widget."
        }
        (true, false) => " Log out and back in to reload the GNOME Shell extension.",
        (false, true) => " Log out and back in to reload the Plasma widget.",
        (false, false) => "",
    };
    Finished {
        version: version.to_string(),
        relogin: gnome || plasma,
        message: format!("Updated to {version}.{reload}"),
    }
}

fn on_path(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
}

fn refuse_without_yes() -> Result<bool> {
    bail!("refusing to update without --yes")
}

fn confirm(question: &str) -> Result<bool> {
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return refuse_without_yes();
    }
    write!(io::stderr(), "{question} [y/N] ")?;
    io::stderr().flush()?;
    let mut answer = String::new();
    stdin.lock().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
