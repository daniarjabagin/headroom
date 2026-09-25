use headroom_core::pace::Tone;

use super::style::tone_hex;

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn bold(markup: &str) -> String {
    format!("<b>{markup}</b>")
}

pub fn toned(markup: &str, tone: Tone) -> String {
    match tone {
        Tone::Warning | Tone::Critical => {
            format!("<span color=\"{}\">{markup}</span>", tone_hex(tone))
        }
        Tone::Good | Tone::Neutral => markup.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_text_is_escaped() {
        assert_eq!(escape("R&D <main>"), "R&amp;D &lt;main&gt;");
    }

    #[test]
    fn only_warning_and_critical_are_colored() {
        assert_eq!(toned("40%", Tone::Good), "40%");
        assert_eq!(toned("—", Tone::Neutral), "—");
        assert_eq!(
            toned("40%", Tone::Warning),
            "<span color=\"#ffd60a\">40%</span>"
        );
        assert_eq!(
            toned("8%", Tone::Critical),
            "<span color=\"#ff453a\">8%</span>"
        );
        assert_eq!(bold("Claude"), "<b>Claude</b>");
    }
}
