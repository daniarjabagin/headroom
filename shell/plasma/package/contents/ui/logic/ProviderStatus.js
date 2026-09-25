.pragma library

.import "I18n.js" as I18n
.import "Parse.js" as Parse

const INDICATORS = ["none", "minor", "major", "critical", "maintenance"];
const TONES = ["neutral", "warning", "critical"];

const INDICATOR_LABELS = {
    minor: I18n.N("Degraded performance"),
    major: I18n.N("Partial outage"),
    critical: I18n.N("Major outage"),
    maintenance: I18n.N("Maintenance")
};

const STAGE_LABELS = {
    investigating: I18n.N("Investigating"),
    identified: I18n.N("Identified"),
    monitoring: I18n.N("Monitoring"),
    in_progress: I18n.N("In progress"),
    verifying: I18n.N("Verifying")
};

function toneFor(indicator) {
    if (indicator === "major" || indicator === "critical")
        return "critical";
    return indicator === "none" ? "neutral" : "warning";
}

function parseStatus(raw) {
    const provider = Parse.text(raw.provider);
    const indicator = Parse.oneOf(INDICATORS, raw.indicator, null);
    if (provider === null || indicator === null)
        return null;
    return {
        provider,
        indicator,
        tone: Parse.oneOf(TONES, raw.tone, toneFor(indicator)),
        title: Parse.text(raw.title),
        stage: Parse.text(raw.stage),
        startedAt: Parse.timestamp(raw.started_at),
        url: Parse.httpsUrl(raw.url)
    };
}

function parseStatuses(raw) {
    return Parse.list(raw).map(parseStatus).filter(status => status !== null);
}

function forProvider(statuses, provider) {
    return statuses.find(status => status.provider === provider) ?? null;
}

function hasIssue(status) {
    return status !== null && status.indicator !== "none";
}

function indicatorLabel(lang, indicator) {
    const msgid = INDICATOR_LABELS[indicator];
    return msgid ? I18n.tr(lang, msgid) : "";
}

function stageLabel(lang, stage) {
    const msgid = STAGE_LABELS[stage];
    return msgid ? I18n.tr(lang, msgid) : stage ?? "";
}
