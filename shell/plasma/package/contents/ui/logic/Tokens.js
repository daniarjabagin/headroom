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

function mix(from, to, amount) {
    return Qt.rgba(from.r + (to.r - from.r) * amount, from.g + (to.g - from.g) * amount, from.b + (to.b - from.b) * amount, 1);
}

function alpha(color, amount) {
    return Qt.rgba(color.r, color.g, color.b, color.a * amount);
}

function surface(theme, amount) {
    return mix(theme.backgroundColor, theme.textColor, amount);
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
    return surface(theme, SECONDARY);
}

function tertiaryText(theme) {
    return surface(theme, TERTIARY);
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
    return mix(card(theme), noticeColor(theme, kind), NOTICE_FILL);
}

function noticeTile(theme, kind) {
    return alpha(noticeColor(theme, kind), NOTICE_TILE);
}

function noticeBorder(theme, kind) {
    return alpha(noticeColor(theme, kind), NOTICE_BORDER);
}
