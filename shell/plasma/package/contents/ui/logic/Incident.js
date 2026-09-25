.pragma library

.import "FormatTime.js" as FormatTime
.import "I18n.js" as I18n
.import "ProviderStatus.js" as ProviderStatus

function kindFor(status) {
    return status.tone === "critical" ? "error" : "warning";
}

function title(lang, status) {
    const label = ProviderStatus.indicatorLabel(lang, status.indicator);
    return status.title === null ? label : `${label} · ${status.title}`;
}

function startedText(lang, status, now) {
    if (status.startedAt === null)
        return "";
    return I18n.tr(lang, "Started {ago}", {
        ago: FormatTime.agoText(lang, status.startedAt, now)
    });
}

function age(lang, status, now) {
    if (status.startedAt === null || status.startedAt > now)
        return "";
    return FormatTime.duration(lang, now - status.startedAt);
}

function notice(lang, status, now) {
    if (!ProviderStatus.hasIssue(status))
        return null;
    return {
        kind: kindFor(status),
        tone: status.tone,
        headline: ProviderStatus.indicatorLabel(lang, status.indicator),
        title: title(lang, status),
        detail: status.stage === null ? "" : ProviderStatus.stageLabel(lang, status.stage),
        started: startedText(lang, status, now),
        age: age(lang, status, now),
        url: status.url
    };
}
