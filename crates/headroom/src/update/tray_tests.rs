use super::*;
use crate::update::method::TrayVariant;

const TRAY: &str = "headroom-tray-0.5.0-x86_64-linux-gnu-layershell";

impl Fixture {
    async fn serve_bundle_with_tray(&self, tray_digest: Option<&str>) {
        let archive = self.bundle_archive();
        let tray = self.tray_archive();
        let tray_digest =
            tray_digest.map_or_else(|| hex::encode(Sha256::digest(&tray)), str::to_owned);
        let sums = format!(
            "{}  {BUNDLE}.tar.gz\n{tray_digest}  {TRAY}.tar.gz\n",
            hex::encode(Sha256::digest(&archive))
        );
        self.serve_sums(&sums, Signed::ByReleaseKey).await;
        self.serve_archive(BUNDLE, archive).await;
        self.serve_archive(TRAY, tray).await;
    }

    fn tray_archive(&self) -> Vec<u8> {
        let source = self.dir.path().join("tray-source");
        let bundle = source.join(TRAY);
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(bundle.join("variant"), "linux-gnu-layershell\n").unwrap();
        tar(&source, TRAY, &self.dir.path().join("tray.tar.gz"))
    }
}

fn script_install_with_tray(options: &[&str]) -> Detected {
    let mut detected = script_install(options);
    if let Some(receipt) = detected.receipt.as_mut() {
        receipt.tray = Some(TrayVariant::LayerShell);
    }
    detected
}

#[tokio::test]
async fn a_recorded_tray_is_downloaded_verified_and_handed_to_the_installer() {
    let tray_asset = format!("{TRAY}.tar.gz");
    let fixture =
        Fixture::with_assets("v0.5.0", &["SHA256SUMS", "SHA256SUMS.sig", &tray_asset]).await;
    fixture.serve_bundle_with_tray(None).await;
    let detected = script_install_with_tray(&["--tray"]);
    let (outcome, events, _) = run_update(&fixture, &detected, true).await;
    outcome.unwrap();
    assert_eq!(read_args(&fixture.args_file()).as_deref(), Some("--tray\n"));
    assert_eq!(
        read_args(&fixture.tray_seen_file()).as_deref(),
        Some("linux-gnu-layershell\n")
    );
    assert_eq!(events.matches("Verifying the checksum").count(), 2);
}

#[tokio::test]
async fn a_tampered_tray_archive_stops_the_update() {
    let tray_asset = format!("{TRAY}.tar.gz");
    let fixture =
        Fixture::with_assets("v0.5.0", &["SHA256SUMS", "SHA256SUMS.sig", &tray_asset]).await;
    fixture.serve_bundle_with_tray(Some(&"0".repeat(64))).await;
    let (outcome, _, _) = run_update(&fixture, &script_install_with_tray(&["--tray"]), true).await;
    let error = outcome.unwrap_err().to_string();
    assert!(
        error.starts_with(&format!("checksum mismatch for {TRAY}.tar.gz")),
        "{error}"
    );
    assert_eq!(read_args(&fixture.args_file()), None);
}

#[tokio::test]
async fn a_release_without_the_recorded_tray_fails_before_downloading() {
    let fixture = Fixture::new("v0.5.0").await;
    fixture.serve_bundle(None).await;
    let (outcome, _, _) = run_update(&fixture, &script_install_with_tray(&["--tray"]), true).await;
    assert_eq!(
        outcome.unwrap_err().to_string(),
        format!(
            "release 0.5.0 has no {TRAY}.tar.gz, so the installed Headroom tray cannot be updated"
        )
    );
    let requested = fixture.server.received_requests().await.unwrap();
    assert_eq!(requested.len(), 1);
    assert_eq!(read_args(&fixture.args_file()), None);
}
