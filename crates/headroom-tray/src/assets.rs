const SYMBOLIC_FILL: &str = "fill=\"#bebebe\"";
const SYMBOLIC_SIZE: &str = "width=\"16\" height=\"16\"";
const SVG_OPEN: &str = "<svg ";
pub const CLAUDE_COLOR: &str = "#D97757";

pub const MARK: &str = include_str!("../../../assets/brand/headroom-symbolic.svg");
pub const FLAME: &str = include_str!("../../../shell/gnome/icons/flame-symbolic.svg");
const GENERIC: &str = include_str!("../../../shell/gnome/icons/provider-symbolic.svg");

const LOGOS: [(&str, &str); 16] = [
    (
        "codex",
        include_str!("../../../assets/providers/openai.svg"),
    ),
    (
        "claude",
        include_str!("../../../assets/providers/claude.svg"),
    ),
    (
        "opencode",
        include_str!("../../../assets/providers/opencode.svg"),
    ),
    (
        "openrouter",
        include_str!("../../../assets/providers/openrouter.svg"),
    ),
    ("zai", include_str!("../../../assets/providers/zdotai.svg")),
    ("kimi", include_str!("../../../assets/providers/kimi.svg")),
    (
        "minimax",
        include_str!("../../../assets/providers/minimax.svg"),
    ),
    ("cline", include_str!("../../../assets/providers/cline.svg")),
    (
        "copilot",
        include_str!("../../../assets/providers/githubcopilot.svg"),
    ),
    (
        "cursor",
        include_str!("../../../assets/providers/cursor.svg"),
    ),
    (
        "antigravity",
        include_str!("../../../assets/providers/google.svg"),
    ),
    (
        "ollama",
        include_str!("../../../assets/providers/ollama.svg"),
    ),
    ("warp", include_str!("../../../assets/providers/warp.svg")),
    ("poe", include_str!("../../../assets/providers/poe.svg")),
    (
        "deepseek",
        include_str!("../../../assets/providers/deepseek.svg"),
    ),
    (
        "moonshot",
        include_str!("../../../assets/providers/moonshotai.svg"),
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    Brand,
    Text,
}

#[must_use]
pub fn provider_logo(provider: &str) -> (&'static str, Tint) {
    let logo = LOGOS
        .iter()
        .find(|(id, _)| *id == provider)
        .map_or(GENERIC, |(_, svg)| *svg);
    let tint = if provider == "claude" {
        Tint::Brand
    } else {
        Tint::Text
    };
    (logo, tint)
}

#[must_use]
pub fn tinted_svg(svg: &str, color: &str, size: u32) -> String {
    let fill = format!("fill=\"{color}\"");
    let dimensions = format!("width=\"{size}\" height=\"{size}\"");
    if svg.contains(SYMBOLIC_FILL) {
        return svg
            .replace(SYMBOLIC_FILL, &fill)
            .replacen(SYMBOLIC_SIZE, &dimensions, 1);
    }
    svg.replacen(SVG_OPEN, &format!("{SVG_OPEN}{fill} {dimensions} "), 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_providers_to_logos() {
        assert!(provider_logo("codex").0.contains("OpenAI"));
        assert_eq!(provider_logo("claude").1, Tint::Brand);
        assert_eq!(provider_logo("grok").0, GENERIC);
        assert_eq!(provider_logo("grok").1, Tint::Text);
        for (provider, title) in [
            ("warp", "Warp"),
            ("poe", "Poe"),
            ("deepseek", "DeepSeek"),
            ("moonshot", "Moonshot"),
        ] {
            assert!(provider_logo(provider).0.contains(title), "{provider}");
        }
        assert_eq!(provider_logo("kilo"), (GENERIC, Tint::Text));
    }

    #[test]
    fn tints_symbolic_and_simple_icons() {
        let mark = tinted_svg(MARK, "#123456", 48);
        assert!(mark.contains("fill=\"#123456\""));
        assert!(mark.contains("width=\"48\" height=\"48\""));
        assert!(!mark.contains("#bebebe"));
        let logo = tinted_svg(provider_logo("codex").0, "#abcdef", 32);
        assert!(logo.starts_with("<svg fill=\"#abcdef\" width=\"32\" height=\"32\" "));
    }
}
