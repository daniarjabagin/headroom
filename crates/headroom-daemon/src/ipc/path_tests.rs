use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

use super::*;

#[test]
fn long_paths_fall_back_to_a_private_temp_dir() {
    let temp = Path::new("/var/folders/xy/T");
    let short = PathBuf::from("/Users/ada/Library/Application Support/Headroom/daemon.sock");
    assert_eq!(
        fit_socket_path(short.clone(), temp, 501),
        SocketPath::Preferred(short)
    );
    let exact = PathBuf::from(format!("/{}", "a".repeat(MAX_SOCKET_PATH - 1)));
    assert_eq!(
        fit_socket_path(exact.clone(), temp, 501),
        SocketPath::Preferred(exact)
    );
    let long = PathBuf::from(format!("/{}", "a".repeat(MAX_SOCKET_PATH)));
    assert_eq!(
        fit_socket_path(long, temp, 501),
        SocketPath::Private {
            dir: PathBuf::from("/var/folders/xy/T/headroom-501"),
            socket: PathBuf::from("/var/folders/xy/T/headroom-501/daemon.sock"),
        }
    );
}

#[test]
fn only_a_0700_directory_of_the_current_user_is_private() {
    let cases = [
        (true, 501, 0o040_700, None),
        (false, 501, 0o100_700, Some(PrivacyIssue::NotADirectory)),
        (
            true,
            0,
            0o040_700,
            Some(PrivacyIssue::Owner { owner: 0, uid: 501 }),
        ),
        (true, 501, 0o040_755, Some(PrivacyIssue::Mode(0o755))),
        (true, 501, 0o041_777, Some(PrivacyIssue::Mode(0o777))),
        (true, 501, 0o040_500, Some(PrivacyIssue::Mode(0o500))),
    ];
    for (is_dir, owner, mode, expected) in cases {
        assert_eq!(
            privacy_issue(is_dir, owner, mode, 501),
            expected,
            "{mode:o}"
        );
    }
}

#[test]
fn the_private_dir_is_created_and_verified() {
    let temp = tempfile::tempdir().unwrap();
    let uid = rustix::process::getuid().as_raw();
    let dir = temp.path().join(format!("headroom-{uid}"));
    ensure_private_dir(&dir, uid).unwrap();
    let mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
    ensure_private_dir(&dir, uid).unwrap();
    fs::set_permissions(&dir, Permissions::from_mode(0o755)).unwrap();
    let error = ensure_private_dir(&dir, uid).unwrap_err();
    assert!(matches!(
        error,
        SocketError::NotPrivate {
            issue: PrivacyIssue::Mode(0o755),
            ..
        }
    ));
    assert!(
        error.to_string().ends_with("its mode is 0755, not 0700"),
        "{error}"
    );
    let squatted = temp.path().join("squatted");
    std::os::unix::fs::symlink(&dir, &squatted).unwrap();
    let error = ensure_private_dir(&squatted, uid).unwrap_err();
    assert!(matches!(
        error,
        SocketError::NotPrivate {
            issue: PrivacyIssue::NotADirectory,
            ..
        }
    ));
    let error = ensure_private_dir(&dir, uid + 1).unwrap_err();
    assert!(matches!(
        error,
        SocketError::NotPrivate {
            issue: PrivacyIssue::Owner { .. },
            ..
        }
    ));
}
