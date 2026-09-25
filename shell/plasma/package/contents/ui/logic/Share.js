.pragma library

.import "Combined.js" as Combined
.import "Commands.js" as Commands
.import "Format.js" as Format
.import "I18n.js" as I18n
.import "Quota.js" as Quota
.import "State.js" as State

const FILE_SCHEME = "file://";
const FOLDER = "Headroom";
const MAX_ROWS = 3;

const PALETTES = {
    light: {
        background: "#F0F0ED",
        foreground: "#151617",
        dim: "#94151617",
        line: "#1F151617",
        card: "#FAFAF8",
        accent: "#A9B5FF"
    },
    dark: {
        background: "#151617",
        foreground: "#F0F0ED",
        dim: "#94F0F0ED",
        line: "#1FF0F0ED",
        card: "#232425",
        accent: "#A9B5FF"
    }
};

const LAYOUT = {
    width: 600,
    height: 315,
    scale: 2,
    padTop: 22,
    padSide: 28,
    padBottom: 18,
    logo: 17,
    tiny: 10,
    body: 12,
    name: 13,
    hero: 58,
    heroSuffix: 22,
    pill: 7,
    pillGap: 3,
    heroMeter: 250,
    cardWidth: 262,
    cardRadius: 12,
    cardPad: 14,
    meter: 5,
    gap: 24,
    small: 6
};

function palette(dark) {
    return PALETTES[dark ? "dark" : "light"];
}

function layout() {
    return LAYOUT;
}
const MONTHS = [I18n.N("Jan"), I18n.N("Feb"), I18n.N("Mar"), I18n.N("Apr"), I18n.N("May"), I18n.N("Jun"), I18n.N("Jul"), I18n.N("Aug"), I18n.N("Sep"), I18n.N("Oct"), I18n.N("Nov"), I18n.N("Dec")];

function twoDigits(value) {
    return String(value).padStart(2, "0");
}

function fileName(provider, date) {
    const day = `${date.getFullYear()}-${twoDigits(date.getMonth() + 1)}-${twoDigits(date.getDate())}`;
    return `headroom-${provider}-${day}-${twoDigits(date.getHours())}${twoDigits(date.getMinutes())}.png`;
}

function localPath(url) {
    const text = String(url ?? "");
    return decodeURIComponent(text.startsWith(FILE_SCHEME) ? text.slice(FILE_SCHEME.length) : text);
}

function folderPath(picturesUrl) {
    const base = localPath(picturesUrl).replace(/\/+$/, "");
    return base === "" ? "" : `${base}/${FOLDER}`;
}

function folderCommand(folder) {
    return `mkdir -p ${Commands.shellQuote(folder)}`;
}

function folderUrl(folder) {
    return `${FILE_SCHEME}${encodeURI(folder)}`;
}

function cardWindows(card) {
    return card.kind === "combined" ? card.group.windows : State.shownWindows(card.account);
}

function remainingShare(window) {
    const capacity = Format.isPooled(window) ? window.capacityPercent : 100;
    return capacity > 0 ? window.remainingPercent / capacity : 0;
}

function heroWindow(card, headline) {
    const windows = cardWindows(card).filter(Quota.hasData);
    const pinned = headline !== null && card.accountIds.includes(headline.accountId) ? windows.find(window => window.id === headline.windowId) : undefined;
    if (pinned)
        return pinned;
    return windows.reduce((lowest, window) => lowest === null || remainingShare(window) < remainingShare(lowest) ? window : lowest, null);
}

function percentOf(window, valueMode) {
    return window.segments !== undefined ? Combined.combinedPercent(window, valueMode) : Quota.shownPercent(window, valueMode);
}

function reading(lang, window, valueMode) {
    const percent = percentOf(window, valueMode);
    if (Format.isPooled(window))
        return Format.capacityReading(lang, percent, window.capacityPercent, valueMode);
    return Format.readingFor(lang, percent, valueMode);
}

function heroSub(lang, window, now, display) {
    const reset = Quota.trailingText(lang, window, now, display.resetFormat, false, display.timeFormat);
    const label = Format.windowLabel(lang, window);
    if (!Format.isPooled(window))
        return `${label} · ${reset}`;
    return I18n.tr(lang, "of {capacity}% · {window} · {reset}", {
        capacity: Format.roundPercent(window.capacityPercent),
        window: label,
        reset
    });
}

function hero(lang, window, now, display) {
    if (window === null)
        return null;
    const percent = percentOf(window, display.valueMode);
    return {
        value: percent === null ? "—" : `${Format.roundPercent(percent)}%`,
        suffix: display.valueMode === "used" ? I18n.tr(lang, "used") : I18n.tr(lang, "left"),
        sub: heroSub(lang, window, now, display),
        window
    };
}

function dateText(lang, date) {
    return I18n.tr(lang, "{day} {month} {year}", {
        day: date.getDate(),
        month: I18n.tr(lang, MONTHS[date.getMonth()]),
        year: date.getFullYear()
    });
}

function windowLine(lang, window, now, display) {
    return {
        window,
        label: Format.windowLabel(lang, window),
        reading: reading(lang, window, display.valueMode),
        reset: Quota.trailingText(lang, window, now, display.resetFormat, false, display.timeFormat)
    };
}

function providerName(card) {
    return card.kind === "combined" ? card.group.providerName : card.account.providerName;
}

function provider(card) {
    return card.kind === "combined" ? card.group.provider : card.account.provider;
}

function model(lang, card, plan, headline, now, display) {
    return {
        provider: provider(card),
        providerName: providerName(card),
        plan: plan ?? "",
        title: I18n.tr(lang, "{provider} limits · {date}", {
            provider: providerName(card),
            date: dateText(lang, now)
        }).toUpperCase(),
        hero: hero(lang, heroWindow(card, headline), now, display),
        rows: cardWindows(card).map(window => windowLine(lang, window, now, display))
    };
}

function meterSegments(window, members, display) {
    if (window.segments !== undefined)
        return Combined.segments(window, members, display);
    return [{
            fraction: Quota.fillFraction(window, display.valueMode),
            tone: Quota.meterTone(window),
            tick: Quota.tickPosition(window, display)
        }];
}

function shownRows(shared) {
    return shared.rows.slice(0, MAX_ROWS);
}

function fractions(window, valueMode) {
    if (window === null)
        return [];
    if (window.segments !== undefined)
        return window.segments.map(segment => Quota.fillFraction(segment, valueMode));
    return [Quota.fillFraction(window, valueMode)];
}

function copyText(shared) {
    const head = shared.plan === "" ? shared.providerName : `${shared.providerName} · ${shared.plan}`;
    return [head].concat(shared.rows.map(row => `${row.label}: ${row.reading} · ${row.reset}`)).join("\n");
}
