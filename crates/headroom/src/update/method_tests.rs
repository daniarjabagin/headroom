use std::fs;

use super::*;

fn receipt(method: ReceiptMethod, prefix: &str) -> Receipt {
    Receipt {
        method,
        options: vec!["--no-plasma".into()],
        prefix: PathBuf::from(prefix),
        tray: None,
    }
}

fn probe(exe: &str, receipt: Option<Receipt>, marker: Option<&str>) -> Probe {
    Probe {
        exe: PathBuf::from(exe),
        receipt,
        marker: marker.map(str::to_owned),
    }
}

#[test]
fn install_methods_follow_the_binary_path_marker_and_receipt() {
    let script = Some(receipt(ReceiptMethod::Script, "/home/ada/.local"));
    let source = Some(receipt(ReceiptMethod::Source, "/home/ada/.local"));
    let cases = [
        (
            probe("/home/ada/.local/bin/headroom", script.clone(), None),
            Install::Script,
        ),
        (
            probe("/home/ada/.local/bin/headroom", source, None),
            Install::Unknown,
        ),
        (
            probe("/home/ada/.local/bin/headroom", None, None),
            Install::Unknown,
        ),
        (
            probe(
                "/home/ada/src/target/release/headroom",
                script.clone(),
                None,
            ),
            Install::Unknown,
        ),
        (
            probe("/usr/bin/headroom", None, Some("deb\n")),
            Install::Package(Packager::Deb),
        ),
        (
            probe("/usr/bin/headroom", None, Some("rpm\n")),
            Install::Package(Packager::Rpm),
        ),
        (
            probe("/usr/bin/headroom", script, Some("archlinux\n")),
            Install::Package(Packager::Arch),
        ),
        (
            probe("/usr/bin/headroom", None, None),
            Install::Package(Packager::Other),
        ),
    ];
    for (probe, expected) in cases {
        assert_eq!(classify(&probe), expected, "{probe:?}");
    }
}

#[test]
fn detection_reads_the_receipt_and_marker_from_disk() {
    let dir = tempfile::tempdir().unwrap();
    let prefix = dir.path().join(".local");
    let bin = prefix.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("headroom"), b"").unwrap();
    let receipt_path = dir.path().join("install.json");
    let receipt_json = serde_json::json!({
        "method": "script",
        "version": "0.4.0",
        "options": ["--no-gnome", "--no-plasma"],
        "prefix": prefix,
    });
    fs::write(&receipt_path, receipt_json.to_string()).unwrap();
    let missing_marker = dir.path().join("marker");
    let detected = detect_at(&bin.join("headroom"), Some(&receipt_path), &missing_marker);
    assert_eq!(detected.install, Install::Script);
    assert_eq!(
        detected.receipt.unwrap().options,
        ["--no-gnome", "--no-plasma"]
    );
    let replaced = PathBuf::from(format!("{} (deleted)", bin.join("headroom").display()));
    let after_update = detect_at(&replaced, Some(&receipt_path), &missing_marker);
    assert_eq!(after_update.install, Install::Script);
}

#[test]
fn an_unreadable_receipt_means_an_unknown_install() {
    let dir = tempfile::tempdir().unwrap();
    let receipt_path = dir.path().join("install.json");
    fs::write(&receipt_path, r#"{"method":"flatpak","prefix":"/app"}"#).unwrap();
    let exe = dir.path().join("headroom");
    let detected = detect_at(&exe, Some(&receipt_path), &dir.path().join("marker"));
    assert_eq!(
        detected,
        Detected {
            install: Install::Unknown,
            receipt: None
        }
    );
    let none = detect_at(&exe, None, &dir.path().join("marker"));
    assert_eq!(none.install, Install::Unknown);
}

#[test]
fn the_tray_variant_is_read_from_the_receipt() {
    let read = |tray: serde_json::Value| {
        let json = serde_json::json!({ "method": "script", "prefix": "/p", "tray": tray });
        serde_json::from_value::<Receipt>(json).unwrap().tray
    };
    assert_eq!(read("linux-gnu".into()), Some(TrayVariant::Portable));
    assert_eq!(
        read("linux-gnu-layershell".into()),
        Some(TrayVariant::LayerShell)
    );
    assert_eq!(read("linux-gnu-future".into()), Some(TrayVariant::Portable));
    assert_eq!(read(serde_json::Value::Null), None);
    let without: Receipt = serde_json::from_str(r#"{"method":"script","prefix":"/p"}"#).unwrap();
    assert_eq!(without.tray, None);
}
