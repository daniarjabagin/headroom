.pragma library

.import "Diagnostics.js" as Diagnostics
.import "I18n.js" as I18n

const FILE_SCHEME = "file://";

function homePath(homeUrl) {
    const text = String(homeUrl ?? "");
    return decodeURIComponent(text.startsWith(FILE_SCHEME) ? text.slice(FILE_SCHEME.length) : text);
}

function folderUrl(logFile, homeUrl) {
    if (!logFile)
        return "";
    const folder = Diagnostics.folderOf(Diagnostics.expandedPath(logFile, homePath(homeUrl)));
    return folder.startsWith("/") ? `${FILE_SCHEME}${encodeURI(folder)}` : "";
}

function logLevelNote(lang, diagnostics) {
    if (diagnostics?.logLevelSource === "env")
        return I18n.tr(lang, "RUST_LOG is set and wins over this setting");
    return I18n.tr(lang, "Debug adds provider responses without tokens");
}

function logFileLine(lang, diagnostics, failure) {
    if (failure !== null)
        return failure;
    if (diagnostics === null)
        return I18n.tr(lang, "Loading…");
    return diagnostics.logFile ?? I18n.tr(lang, "The service writes no log file");
}
