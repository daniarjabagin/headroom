.pragma library

const CARD = 0.03;
const CARD_HOVER = 0.06;
const CHIP = 0.08;
const CONTROL = 0.07;
const TRACK = 0.13;
const SEPARATOR = 0.1;
const SECONDARY = 0.6;
const TERTIARY = 0.3;
const TICK = 0.55;
const NOTICE_FILL = 0.12;
const NOTICE_TILE = 0.16;
const NOTICE_BORDER = 0.22;
const PANEL_TRACK = 0.28;
const CONTROL_SHADOW = 0.18;
const SKELETON = 0.09;
const SHIMMER = 0.1;
const TRANSLUCENT_ALPHA = 0.72;
const FORCED = {
    light: {
        backgroundColor: "#ffffff",
        textColor: "#262626"
    },
    dark: {
        backgroundColor: "#1e1e1e",
        textColor: "#dedede"
    }
};

function mixAlpha(from, to, amount, alphaValue) {
    return Qt.rgba(from.r + (to.r - from.r) * amount, from.g + (to.g - from.g) * amount, from.b + (to.b - from.b) * amount, alphaValue);
}

function mix(from, to, amount) {
    return mixAlpha(from, to, amount, 1);
}

function alpha(color, amount) {
    return Qt.rgba(color.r, color.g, color.b, color.a * amount);
}

function surface(theme, amount) {
    return mixAlpha(theme.backgroundColor, theme.textColor, amount, theme.backgroundColor.a);
}

function card(theme) {
    return surface(theme, CARD);
}

function cardHover(theme) {
    return surface(theme, CARD_HOVER);
}

function chip(theme) {
    return surface(theme, CHIP);
}

function control(theme) {
    return surface(theme, CONTROL);
}

function track(theme) {
    return surface(theme, TRACK);
}

function separator(theme) {
    return alpha(theme.textColor, SEPARATOR);
}

function secondaryText(theme) {
    return mix(theme.backgroundColor, theme.textColor, SECONDARY);
}

function tertiaryText(theme) {
    return mix(theme.backgroundColor, theme.textColor, TERTIARY);
}

function skeletonGlow(theme, glow) {
    return alpha(theme.textColor, SKELETON + SHIMMER * glow);
}

function forcedColors(themeMode) {
    return FORCED[themeMode] ?? null;
}

function popupPalette(system, themeMode, translucent) {
    const forced = forcedColors(themeMode);
    if (forced === null && !translucent)
        return null;
    const background = Qt.lighter(forced?.backgroundColor ?? system.backgroundColor, 1);
    return {
        backgroundColor: Qt.rgba(background.r, background.g, background.b, translucent ? TRANSLUCENT_ALPHA : 1),
        textColor: forced?.textColor ?? system.textColor,
        painted: forced !== null
    };
}

function tick(theme) {
    return alpha(theme.textColor, TICK);
}

function panelTrack(theme) {
    return alpha(theme.textColor, PANEL_TRACK);
}

function toneColor(theme, tone) {
    if (tone === "critical")
        return theme.negativeTextColor;
    if (tone === "warning")
        return theme.neutralTextColor;
    if (tone === "good")
        return theme.highlightColor;
    return "transparent";
}

function controlShadow() {
    return Qt.rgba(0, 0, 0, CONTROL_SHADOW);
}

function noticeColor(theme, kind) {
    return kind === "error" ? theme.negativeTextColor : theme.neutralTextColor;
}

function noticeFill(theme, kind) {
    return mixAlpha(card(theme), noticeColor(theme, kind), NOTICE_FILL, theme.backgroundColor.a);
}

function noticeTile(theme, kind) {
    return alpha(noticeColor(theme, kind), NOTICE_TILE);
}

function noticeBorder(theme, kind) {
    return alpha(noticeColor(theme, kind), NOTICE_BORDER);
}
