use std::path::Path;

const MACOS_VERSION_PLIST: &str = "/System/Library/CoreServices/SystemVersion.plist";
const OS_RELEASE_FILES: [&str; 2] = ["/etc/os-release", "/usr/lib/os-release"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemInfo {
    pub os: Option<String>,
    pub desktop: Option<String>,
}

#[must_use]
pub fn detect() -> SystemInfo {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
    let session = std::env::var("XDG_SESSION_TYPE").ok();
    SystemInfo {
        os: os_name(),
        desktop: desktop_label(desktop.as_deref(), session.as_deref()),
    }
}

fn os_name() -> Option<String> {
    read(Path::new(MACOS_VERSION_PLIST))
        .and_then(|plist| macos_version(&plist))
        .or_else(|| {
            OS_RELEASE_FILES
                .iter()
                .find_map(|path| read(Path::new(path)).and_then(|text| pretty_name(&text)))
        })
}

fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

pub(crate) fn pretty_name(os_release: &str) -> Option<String> {
    let value = |key: &str| {
        os_release
            .lines()
            .find_map(|line| line.trim().strip_prefix(key)?.strip_prefix('='))
            .map(unquote)
            .filter(|value| !value.is_empty())
    };
    value("PRETTY_NAME").or_else(|| value("NAME"))
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
        })
        .unwrap_or(trimmed)
        .to_owned()
}

pub(crate) fn macos_version(plist: &str) -> Option<String> {
    let after_key = plist.split_once("<key>ProductVersion</key>")?.1;
    let start = after_key.split_once("<string>")?.1;
    let version = start.split_once("</string>")?.0.trim();
    (!version.is_empty()).then(|| format!("macOS {version}"))
}

pub(crate) fn desktop_label(desktop: Option<&str>, session: Option<&str>) -> Option<String> {
    let desktop = desktop.map(str::trim).filter(|d| !d.is_empty());
    let session = session.map(str::trim).filter(|s| !s.is_empty());
    match (desktop, session) {
        (Some(desktop), Some(session)) => Some(format!("{desktop} ({session})")),
        (Some(desktop), None) => Some(desktop.to_owned()),
        (None, Some(session)) => Some(format!("({session})")),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_release_prefers_the_pretty_name() {
        let arch = "NAME=\"Arch Linux\"\nPRETTY_NAME=\"Arch Linux\"\nID=arch\n";
        assert_eq!(pretty_name(arch).as_deref(), Some("Arch Linux"));
        let ubuntu = "PRETTY_NAME='Ubuntu 24.04.1 LTS'\nNAME=Ubuntu\n";
        assert_eq!(pretty_name(ubuntu).as_deref(), Some("Ubuntu 24.04.1 LTS"));
        assert_eq!(pretty_name("NAME=Fedora\n").as_deref(), Some("Fedora"));
        assert_eq!(pretty_name("PRETTY_NAME=\"\"\nID=x\n"), None);
        assert_eq!(pretty_name("PRETTY_NAME_EXTRA=x\n"), None);
    }

    #[test]
    fn the_macos_version_comes_from_the_product_version_key() {
        let plist = "<dict>\n\t<key>ProductName</key>\n\t<string>macOS</string>\n\
                     \t<key>ProductVersion</key>\n\t<string>27.0</string>\n</dict>";
        assert_eq!(macos_version(plist).as_deref(), Some("macOS 27.0"));
        assert_eq!(macos_version("<dict></dict>"), None);
    }

    #[test]
    fn the_desktop_names_the_session_type() {
        assert_eq!(
            desktop_label(Some("GNOME"), Some("wayland")).as_deref(),
            Some("GNOME (wayland)")
        );
        assert_eq!(desktop_label(Some("KDE"), None).as_deref(), Some("KDE"));
        assert_eq!(desktop_label(None, Some("x11")).as_deref(), Some("(x11)"));
        assert_eq!(desktop_label(Some(" "), Some("")), None);
    }
}
