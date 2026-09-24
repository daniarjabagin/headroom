const GNOME_SHELL_SESSIONS: [&str; 5] = ["GNOME", "ubuntu", "pop", "Zorin", "GNOME-Classic"];
const PLASMA: &str = "KDE";

#[must_use]
pub fn has_own_shell(current_desktop: Option<&str>) -> bool {
    let tokens: Vec<&str> = current_desktop
        .unwrap_or_default()
        .split(':')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect();
    let plasma = tokens.contains(&PLASMA);
    let gnome_shell = tokens
        .first()
        .is_some_and(|first| GNOME_SHELL_SESSIONS.contains(first));
    plasma || gnome_shell
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnome_shell_and_plasma_have_their_own_shell() {
        for desktop in [
            "GNOME",
            "ubuntu:GNOME",
            "pop:GNOME",
            "GNOME-Classic:GNOME",
            "KDE",
        ] {
            assert!(has_own_shell(Some(desktop)), "{desktop}");
        }
    }

    #[test]
    fn every_other_desktop_needs_the_tray() {
        for desktop in [
            "Budgie:GNOME",
            "GNOME-Flashback:GNOME",
            "X-Cinnamon",
            "XFCE",
            "MATE",
            "LXQt",
            "Hyprland",
            "sway",
            "niri",
            "i3",
            "COSMIC",
            "",
        ] {
            assert!(!has_own_shell(Some(desktop)), "{desktop}");
        }
        assert!(!has_own_shell(None));
    }
}
