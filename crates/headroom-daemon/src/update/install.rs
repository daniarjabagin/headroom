use serde::{Deserialize, Serialize};

use super::release::Release;

pub const SELF_UPDATE_COMMAND: &str = "headroom update";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Install {
    Script,
    Package(Packager),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Packager {
    Deb,
    Rpm,
    Arch,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallKind {
    #[serde(rename = "self")]
    SelfInstalled,
    Package,
    Unknown,
}

impl Packager {
    #[must_use]
    pub fn from_marker(text: &str) -> Packager {
        match text.trim() {
            "deb" => Packager::Deb,
            "rpm" => Packager::Rpm,
            "archlinux" => Packager::Arch,
            _ => Packager::Other,
        }
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Packager::Deb => "deb package",
            Packager::Rpm => "rpm package",
            Packager::Arch => "Arch package",
            Packager::Other => "system package",
        }
    }
}

impl Install {
    #[must_use]
    pub fn kind(self) -> InstallKind {
        match self {
            Install::Script => InstallKind::SelfInstalled,
            Install::Package(_) => InstallKind::Package,
            Install::Unknown => InstallKind::Unknown,
        }
    }

    #[must_use]
    pub fn command(self, release: &Release) -> String {
        let url = &release.url;
        match self {
            Install::Script => SELF_UPDATE_COMMAND.to_owned(),
            Install::Package(Packager::Deb) => format!("Download the new .deb package from {url}"),
            Install::Package(Packager::Rpm) => format!("Download the new .rpm package from {url}"),
            Install::Package(Packager::Arch) => {
                format!(
                    "Download the new Arch package from {url} and install it with sudo pacman -U"
                )
            }
            Install::Package(Packager::Other) => {
                "Update headroom with your system package manager".to_owned()
            }
            Install::Unknown => url.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release() -> Release {
        Release {
            version: "0.5.0".parse().unwrap(),
            url: "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0".into(),
            published_at: "2026-10-01T09:20:02Z".parse().unwrap(),
        }
    }

    #[test]
    fn every_install_method_has_one_command() {
        let url = "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0";
        let cases = [
            (Install::Script, "headroom update".to_owned()),
            (
                Install::Package(Packager::Deb),
                format!("Download the new .deb package from {url}"),
            ),
            (
                Install::Package(Packager::Rpm),
                format!("Download the new .rpm package from {url}"),
            ),
            (
                Install::Package(Packager::Arch),
                format!(
                    "Download the new Arch package from {url} and install it with sudo pacman -U"
                ),
            ),
            (
                Install::Package(Packager::Other),
                "Update headroom with your system package manager".to_owned(),
            ),
            (Install::Unknown, url.to_owned()),
        ];
        for (install, command) in cases {
            assert_eq!(install.command(&release()), command, "{install:?}");
        }
    }

    #[test]
    fn kinds_serialize_to_the_contract_strings() {
        let text = |install: Install| serde_json::to_string(&install.kind()).unwrap();
        assert_eq!(text(Install::Script), r#""self""#);
        assert_eq!(text(Install::Package(Packager::Rpm)), r#""package""#);
        assert_eq!(text(Install::Unknown), r#""unknown""#);
    }

    #[test]
    fn markers_name_the_packager() {
        assert_eq!(Packager::from_marker("deb\n"), Packager::Deb);
        assert_eq!(Packager::from_marker("rpm"), Packager::Rpm);
        assert_eq!(Packager::from_marker("archlinux\n"), Packager::Arch);
        assert_eq!(Packager::from_marker("aur"), Packager::Other);
    }
}
