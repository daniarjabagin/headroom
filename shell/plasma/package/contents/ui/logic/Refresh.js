.pragma library

.import "Parse.js" as Parse

const MODES = ["live", "idle"];
const REASONS = ["activity", "schedule", "backoff", "hold"];
const MIN_INTERVAL_SECS = 60;

function interval(value) {
    const seconds = Parse.integer(value);
    return seconds === null ? null : Math.max(MIN_INTERVAL_SECS, seconds);
}

function parseRefresh(raw) {
    if (!Parse.isObject(raw))
        return null;
    const mode = Parse.oneOf(MODES, raw.mode, "idle");
    return {
        mode,
        intervalSecs: interval(raw.interval_secs),
        nextAt: Parse.timestamp(raw.next_at),
        reason: Parse.oneOf(REASONS, raw.reason, mode === "live" ? "activity" : "schedule")
    };
}

function isLive(account) {
    return account.refresh?.mode === "live";
}
