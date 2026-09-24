use std::path::{Path, PathBuf};
use std::process::Command;

use headroom_daemon::update::Packager;
use sha2::{Digest, Sha256};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use method::{Receipt, ReceiptMethod};

const BUNDLE: &str = "headroom-0.5.0-x86_64-linux-musl";

struct Fixture {
    server: MockServer,
    dir: tempfile::TempDir,
    feed: GithubFeed,
}

impl Fixture {
    async fn new(tag: &str) -> Fixture {
        let server = MockServer::start().await;
        let feed = GithubFeed::new(reqwest::Client::new(), format!("{}/latest", server.uri()));
        let release = release_json(&server.uri(), tag);
        Mock::given(path("/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(release))
            .mount(&server)
            .await;
        Fixture {
            server,
            dir: tempfile::tempdir().unwrap(),
            feed,
        }
    }

    fn args_file(&self) -> PathBuf {
        self.dir.path().join("install-args")
    }

    async fn serve_bundle(&self, listed_digest: Option<&str>) {
        let archive = self.bundle_archive();
        let digest =
            listed_digest.map_or_else(|| hex::encode(Sha256::digest(&archive)), str::to_owned);
        let sums = format!("{digest}  {BUNDLE}.tar.gz\n");
        Mock::given(path("/SHA256SUMS"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sums))
            .mount(&self.server)
            .await;
        Mock::given(path(format!("/{BUNDLE}.tar.gz")))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(archive))
            .mount(&self.server)
            .await;
    }

    fn bundle_archive(&self) -> Vec<u8> {
        let source = self.dir.path().join("source");
        let bundle = source.join(BUNDLE);
        std::fs::create_dir_all(&bundle).unwrap();
        let script = format!(
            "#!/usr/bin/env bash\nset -eu\necho '==> Installing the binary'\necho 'plain output'\nprintf '%s\\n' \"$@\" > '{}'\n",
            self.args_file().display()
        );
        std::fs::write(bundle.join("install.sh"), script).unwrap();
        let archive = self.dir.path().join("bundle.tar.gz");
        let status = Command::new("tar")
            .arg("-czf")
            .arg(&archive)
            .arg("-C")
            .arg(&source)
            .arg(BUNDLE)
            .status()
            .unwrap();
        assert!(status.success());
        std::fs::read(archive).unwrap()
    }
}

fn release_json(base: &str, tag: &str) -> serde_json::Value {
    let asset = |name: &str| serde_json::json!({ "name": name, "browser_download_url": format!("{base}/{name}") });
    serde_json::json!({
        "tag_name": tag,
        "html_url": format!("https://github.com/daniarjabagin/headroom/releases/tag/{tag}"),
        "draft": false,
        "prerelease": false,
        "published_at": "2026-10-01T09:20:02Z",
        "assets": [asset("SHA256SUMS"), asset(&format!("{BUNDLE}.tar.gz")), asset("headroom_0.5.0-1_amd64.deb")]
    })
}

fn script_install(options: &[&str]) -> Detected {
    Detected {
        install: Install::Script,
        receipt: Some(Receipt {
            method: ReceiptMethod::Script,
            options: options.iter().map(|option| (*option).to_owned()).collect(),
            prefix: PathBuf::from("/home/ada/.local"),
        }),
    }
}

fn gnome_only(program: &str) -> bool {
    program == "gnome-shell"
}

async fn run_update(
    fixture: &Fixture,
    detected: &Detected,
    approved: bool,
) -> (Result<Finished>, String) {
    let current: Version = "0.4.0".parse().unwrap();
    let updater = Updater {
        feed: &fixture.feed,
        current: &current,
        arch: "x86_64",
        detected,
        on_path: &gnome_only,
    };
    let mut reporter = Reporter::Json(JsonLines::new(Vec::new()));
    let outcome = update(&updater, |_| Ok(approved), &mut reporter).await;
    let Reporter::Json(lines) = reporter else {
        unreachable!("the reporter is JSON");
    };
    (outcome, String::from_utf8(lines.into_inner()).unwrap())
}

fn read_args(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[tokio::test]
async fn a_verified_bundle_runs_its_installer_with_the_receipt_options() {
    let fixture = Fixture::new("v0.5.0").await;
    fixture.serve_bundle(None).await;
    let detected = script_install(&["--no-plasma"]);
    let (outcome, events) = run_update(&fixture, &detected, true).await;
    let finished = outcome.unwrap();
    assert_eq!(
        finished,
        Finished {
            version: "0.5.0".into(),
            relogin: true,
            message: "Updated to 0.5.0. Log out and back in to reload the GNOME Shell extension."
                .into(),
        }
    );
    assert_eq!(
        read_args(&fixture.args_file()).as_deref(),
        Some("--no-plasma\n")
    );
    let expected = [
        r#"{"event":"step","text":"Checking for a new release…"}"#,
        r#"{"event":"step","text":"Downloading Headroom 0.5.0…"}"#,
        r#"{"event":"step","text":"Verifying the checksum…"}"#,
        r#"{"event":"step","text":"Unpacking…"}"#,
        r#"{"event":"step","text":"Installing the binary"}"#,
    ];
    assert_eq!(events, expected.join("\n") + "\n");
}

#[tokio::test]
async fn a_checksum_mismatch_aborts_before_installing() {
    let fixture = Fixture::new("v0.5.0").await;
    fixture.serve_bundle(Some(&"0".repeat(64))).await;
    let (outcome, _) = run_update(&fixture, &script_install(&[]), true).await;
    let error = outcome.unwrap_err().to_string();
    assert!(
        error.starts_with("checksum mismatch for headroom-0.5.0-x86_64-linux-musl.tar.gz"),
        "{error}"
    );
    assert_eq!(read_args(&fixture.args_file()), None);
}

#[tokio::test]
async fn package_and_unknown_installs_are_told_what_to_do() {
    let fixture = Fixture::new("v0.5.0").await;
    let deb = Detected {
        install: Install::Package(Packager::Deb),
        receipt: None,
    };
    let (outcome, _) = run_update(&fixture, &deb, true).await;
    assert_eq!(
        outcome.unwrap_err().to_string(),
        "Headroom was installed from a deb package, so it cannot update itself. \
         Download the new .deb package from https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0"
    );
    let unknown = Detected {
        install: Install::Unknown,
        receipt: None,
    };
    let (outcome, _) = run_update(&fixture, &unknown, true).await;
    assert!(
        outcome
            .unwrap_err()
            .to_string()
            .contains("not installed by install.sh")
    );
}

#[tokio::test]
async fn an_up_to_date_install_downloads_nothing() {
    let fixture = Fixture::new("v0.4.0").await;
    let (outcome, _) = run_update(&fixture, &script_install(&[]), true).await;
    assert_eq!(
        outcome.unwrap(),
        Finished {
            version: "0.4.0".into(),
            relogin: false,
            message: "Headroom 0.4.0 is up to date.".into(),
        }
    );
    let requests = fixture.server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
}

#[tokio::test]
async fn a_declined_update_changes_nothing() {
    let fixture = Fixture::new("v0.5.0").await;
    fixture.serve_bundle(None).await;
    let (outcome, _) = run_update(&fixture, &script_install(&[]), false).await;
    assert_eq!(outcome.unwrap_err().to_string(), "update cancelled");
    assert_eq!(read_args(&fixture.args_file()), None);
}

#[tokio::test]
async fn check_compares_the_running_version_with_the_latest() {
    let fixture = Fixture::new("v0.5.0").await;
    let detected = script_install(&[]);
    let current: Version = "0.4.0".parse().unwrap();
    let updater = Updater {
        feed: &fixture.feed,
        current: &current,
        arch: "x86_64",
        detected: &detected,
        on_path: &gnome_only,
    };
    assert_eq!(
        check(&updater).await.unwrap(),
        "Headroom 0.4.0 is installed; 0.5.0 is available (2026-10-01).\nUpdate with: headroom update"
    );
    let newest: Version = "0.5.0".parse().unwrap();
    let updater = Updater {
        current: &newest,
        ..updater
    };
    assert_eq!(
        check(&updater).await.unwrap(),
        "Headroom 0.5.0 is up to date."
    );
}

#[test]
fn the_reload_note_names_the_shells_that_were_installed() {
    let version: Version = "0.5.0".parse().unwrap();
    let everything = |_: &str| true;
    let nothing = |_: &str| false;
    let both = finished(&version, &[], &everything);
    assert!(both.relogin);
    assert!(
        both.message
            .ends_with("the GNOME Shell extension and the Plasma widget.")
    );
    let plasma = finished(&version, &["--no-gnome".to_owned()], &everything);
    assert!(plasma.message.ends_with("reload the Plasma widget."));
    let none = finished(&version, &[], &nothing);
    assert_eq!(
        (none.relogin, none.message.as_str()),
        (false, "Updated to 0.5.0.")
    );
}
