.pragma library

.import "Format.js" as Format
.import "Motion.js" as Motion

const KEY_SEPARATOR = "\t";
const PULSE_FACTOR = 10;
const PULSE_OPACITY = 140 / 255;
const STALE_OPACITY = 140 / 255;
const TICK_ALPHA = 0.8;
const PLACEHOLDER = Object.freeze({
    accountId: null,
    windowId: null,
    windowLabel: null,
    provider: "unknown",
    logo: "unknown",
    tone: "neutral",
    valuePercent: 0,
    evenPacePercent: null,
    combined: false,
    accountCount: null
});

function showsMark(items, display) {
    if (display.panelMode === "icon" || items.length === 0)
        return true;
    return display.panelIndicator === "none" && display.panelLabel === "none";
}

function markTone(display, panelTone) {
    return display.panelMode === "icon" ? panelTone ?? "neutral" : "neutral";
}

function parts(display, vertical) {
    const logo = display.panelMode === "several" || display.panelLabel === "window";
    return {
        logo,
        letter: logo && !vertical,
        indicator: display.panelIndicator,
        value: display.panelLabel !== "none",
        tintedValue: display.panelIndicator === "none"
    };
}

function worstItem(items) {
    return items.find(item => item.tone === "critical") ?? items.find(item => item.tone === "warning") ?? items[0];
}

function thinLimit(units) {
    return units.gridUnit * 2 + units.smallSpacing;
}

function shownItems(items, vertical, thickness, units) {
    if (vertical && items.length > 1 && thickness < thinLimit(units))
        return [worstItem(items)];
    return items;
}

function itemAt(items, index) {
    return items[index] ?? PLACEHOLDER;
}

function itemKey(item) {
    return `${item.accountId ?? item.provider}\n${item.windowId ?? ""}`;
}

function keys(items) {
    return items.map(itemKey).join(KEY_SEPARATOR);
}

function keyList(joined) {
    return joined === "" ? [] : joined.split(KEY_SEPARATOR);
}

function valueText(item) {
    return Format.panelPercent(item.valuePercent);
}

function showsCount(item) {
    return item.combined === true && typeof item.accountCount === "number";
}

function fraction(item) {
    return Math.min(1, Math.max(0, item.valuePercent / 100));
}

function tickPosition(item, valueMode) {
    const even = item.evenPacePercent;
    if (even === null || even === undefined)
        return null;
    const position = valueMode === "used" ? even / 100 : 1 - even / 100;
    return Math.min(1, Math.max(0, position));
}

function toneColor(theme, tone) {
    if (tone === "critical")
        return theme.negativeTextColor;
    if (tone === "warning")
        return theme.neutralTextColor;
    return theme.textColor;
}

function tickColor(theme) {
    const color = theme.textColor;
    return Qt.rgba(color.r, color.g, color.b, color.a * TICK_ALPHA);
}

function valueColor(theme, tone, tinted) {
    return tinted ? toneColor(theme, tone) : theme.textColor;
}

function pulses(tone, units, reduced) {
    return tone === "critical" && Motion.enabled(units, reduced);
}

function pulsePeriod(units) {
    return units.longDuration * PULSE_FACTOR;
}

function pulseOpacity() {
    return PULSE_OPACITY;
}

function staleOpacity() {
    return STALE_OPACITY;
}
