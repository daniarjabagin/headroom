.pragma library

.import "FormatTime.js" as FormatTime
.import "I18n.js" as I18n

const STATUSES = ["up_to_date", "available", "failed", "rate_limited", "disabled"];
const MINIMUM_VERSION = [0, 6, 0];
const RETRY_ERRORS = ["org.freedesktop.DBus.Error.NoReply", "org.freedesktop.DBus.Error.Timeout"];
const NOT_SUPPORTED = "org.freedesktop.DBus.Error.NotSupported";
const SERVICE_UNKNOWN = "org.freedesktop.DBus.Error.ServiceUnknown";
const MAX_ATTEMPTS = 3;
const KEPT_RESULTS = ["failed", "rate_limited", "disabled"];

function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function text(value) {
    return typeof value === "string" && value.trim().length > 0 ? value.trim() : null;
}

function timestamp(value) {
    const ms = typeof value === "string" ? Date.parse(value) : NaN;
    return Number.isNaN(ms) ? null : new Date(ms);
}

function parseUpdateCheck(raw) {
    if (!isObject(raw))
        return null;
    return {
        checkedAt: timestamp(raw.checked_at)
    };
}

function result(status, checkedAt, version, until) {
    return {
        status,
        checkedAt,
        version,
        until
    };
}

function failed(checkedAt) {
    return result("failed", checkedAt, null, null);
}

function parseJson(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        return null;
    }
}

function parseResult(json, previousCheckedAt) {
    const raw = parseJson(json);
    if (!isObject(raw) || !STATUSES.includes(raw.status))
        return failed(previousCheckedAt);
    return result(raw.status, timestamp(raw.checked_at), text(raw.version), timestamp(raw.until));
}

function outcome(errorName, value, previousCheckedAt) {
    if (errorName === "")
        return parseResult(value, previousCheckedAt);
    if (errorName === NOT_SUPPORTED)
        return result("disabled", null, null, null);
    return failed(previousCheckedAt);
}

function retries(errorName, attempt) {
    return RETRY_ERRORS.includes(errorName) && attempt < MAX_ATTEMPTS;
}

function versionParts(version) {
    const match = /^v?(\d+)\.(\d+)\.(\d+)/.exec(version ?? "");
    return match === null ? null : [Number(match[1]), Number(match[2]), Number(match[3])];
}

function supported(appVersion) {
    const parts = versionParts(appVersion);
    if (parts === null)
        return false;
    const differing = parts.findIndex((part, index) => part !== MINIMUM_VERSION[index]);
    return differing === -1 || parts[differing] > MINIMUM_VERSION[differing];
}

function visible(checksOn, snapshot) {
    return checksOn && snapshot !== null && snapshot.updateCheck !== null && supported(snapshot.appVersion);
}

function fromState(snapshot) {
    const checkedAt = snapshot.updateCheck?.checkedAt ?? null;
    if (snapshot.update !== null)
        return result("available", checkedAt, snapshot.update.version, null);
    return result(checkedAt === null ? "unchecked" : "up_to_date", checkedAt, null, null);
}

function shown(snapshot, lastResult) {
    if (lastResult !== null && KEPT_RESULTS.includes(lastResult.status))
        return lastResult;
    return fromState(snapshot);
}

function checked(lang, checkedAt, now) {
    return I18n.tr(lang, "checked {ago}", {
        ago: FormatTime.agoText(lang, checkedAt, now)
    });
}

function rateLimitedLine(lang, until, now) {
    if (until === null || until <= now)
        return I18n.tr(lang, "GitHub limits update checks · try again later");
    return I18n.tr(lang, "GitHub limits update checks · try again in {duration}", {
        duration: FormatTime.duration(lang, until - now)
    });
}

function upToDateLine(lang, appVersion, checkedAt, now) {
    return I18n.tr(lang, "You're up to date · Headroom {version} · {checked}", {
        version: appVersion,
        checked: checked(lang, checkedAt, now)
    });
}

function availableLine(lang, checkedAt, now) {
    if (checkedAt === null)
        return I18n.tr(lang, "Update available");
    return I18n.tr(lang, "Update available · {checked}", {
        checked: checked(lang, checkedAt, now)
    });
}

function failedLine(lang, checkedAt, now) {
    if (checkedAt === null)
        return I18n.tr(lang, "Couldn't check for updates");
    return I18n.tr(lang, "Couldn't check for updates · last checked {ago}", {
        ago: FormatTime.agoText(lang, checkedAt, now)
    });
}

function statusLine(lang, check, appVersion, now) {
    switch (check.status) {
    case "up_to_date":
        return check.checkedAt === null ? I18n.tr(lang, "You're up to date · Headroom {version}", {
            version: appVersion
        }) : upToDateLine(lang, appVersion, check.checkedAt, now);
    case "available":
        return availableLine(lang, check.checkedAt, now);
    case "failed":
        return failedLine(lang, check.checkedAt, now);
    case "rate_limited":
        return rateLimitedLine(lang, check.until, now);
    case "disabled":
        return I18n.tr(lang, "Update checks are off");
    default:
        return I18n.tr(lang, "Headroom {version} · not checked yet", {
            version: appVersion
        });
    }
}
