.pragma library

.import "Parse.js" as Parse

const LOG_LEVELS = ["error", "warn", "info", "debug", "trace", "off"];
const LOG_LEVEL_SOURCES = ["settings", "env"];

function decode(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        return null;
    }
}

function parseDiagnostics(json) {
    const raw = decode(json);
    if (!Parse.isObject(raw) || typeof raw.text !== "string")
        return null;
    return {
        appVersion: Parse.text(raw.app_version),
        os: Parse.text(raw.os),
        desktop: Parse.text(raw.desktop),
        uptimeSecs: Parse.number(raw.uptime_secs),
        logLevel: Parse.oneOf(LOG_LEVELS, raw.log_level, null),
        logLevelSource: Parse.oneOf(LOG_LEVEL_SOURCES, raw.log_level_source, null),
        logFile: Parse.text(raw.log_file),
        text: raw.text
    };
}

function expandedPath(path, home) {
    if (path === "~")
        return home;
    return path.startsWith("~/") ? `${home.replace(/\/+$/, "")}${path.slice(1)}` : path;
}

function folderOf(path) {
    const slash = path.lastIndexOf("/");
    if (slash < 0)
        return path;
    return slash === 0 ? "/" : path.slice(0, slash);
}
