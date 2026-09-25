.pragma library

.import "I18n.js" as I18n
.import "Options.js" as Options
.import "Settings.js" as Settings
.import "SettingsValues.js" as Values

const LOOSE_CLOCK = /^(\d{1,2}):(\d{2})$/;

function milestoneSubtitle(lang, milestone, threshold, capable) {
    if (capable && milestone.key === "almostOut")
        return I18n.tr(lang, "A limit drops under {percent}% left", {
            percent: threshold
        });
    return I18n.tr(lang, milestone.subtitle);
}

function providerRows(state) {
    const rows = [];
    for (const account of state?.accounts ?? [])
        if (!rows.some(row => row.provider === account.provider))
            rows.push({
                provider: account.provider,
                name: account.providerName
            });
    return rows;
}

function differingCount(thresholds, rows) {
    return rows.filter(row => row.provider in thresholds).length;
}

function perProviderSummary(lang, thresholds, rows) {
    const count = differingCount(thresholds, rows);
    if (count === 0)
        return I18n.tr(lang, "Every provider uses the default");
    return I18n.trn(lang, "{count} provider differs from the default", "{count} providers differ from the default", count);
}

function thresholdPatch(percent) {
    return Settings.notificationsPatch({
        thresholdPercent: percent
    });
}

function providerPatch(provider, value) {
    return Settings.providerThresholdPatch(provider, Options.providerThresholdFor(value));
}

function quietPatch(fields) {
    return Settings.notificationsPatch({
        quietHours: fields
    });
}

function normalizedClock(text) {
    const match = LOOSE_CLOCK.exec(String(text ?? "").trim());
    if (match === null)
        return null;
    const clock = `${match[1].padStart(2, "0")}:${match[2]}`;
    return Values.isClock(clock) ? clock : null;
}

function clockPatch(hours, key, text) {
    const clock = normalizedClock(text);
    if (clock === null || clock === hours[key])
        return null;
    const next = Object.assign({}, hours, {
        [key]: clock
    });
    const fields = {
        [key]: clock
    };
    if (hours.enabled && !Values.canEnableQuietHours(next))
        fields.enabled = false;
    return quietPatch(fields);
}

function canEnable(hours) {
    return Values.canEnableQuietHours(hours);
}
