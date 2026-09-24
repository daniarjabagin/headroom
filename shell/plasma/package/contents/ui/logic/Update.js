.pragma library

.import "I18n.js" as I18n

const INSTALL_KINDS = ["self", "package", "unknown"];
const RELEASE_URL = /^https:\/\/\S+$/;
const COMMAND = "headroom update --yes --progress json";
const IDLE = Object.freeze({
    phase: "idle"
});

function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function text(value) {
    return typeof value === "string" && value.trim().length > 0 ? value.trim() : null;
}

function releaseUrl(value) {
    const url = text(value);
    return url !== null && RELEASE_URL.test(url) ? url : null;
}

function parseUpdate(raw) {
    if (!isObject(raw))
        return null;
    const version = text(raw.version);
    if (version === null)
        return null;
    const published = typeof raw.published_at === "string" ? Date.parse(raw.published_at) : NaN;
    return {
        version,
        url: releaseUrl(raw.url),
        publishedAt: Number.isNaN(published) ? null : new Date(published),
        install: INSTALL_KINDS.includes(raw.install) ? raw.install : "unknown",
        command: text(raw.command)
    };
}

function action(update) {
    if (update.install === "self")
        return "install";
    if (update.install === "package" && update.command !== null)
        return "command";
    return update.url !== null ? "notes" : "";
}

function actionLabel(lang, kind) {
    switch (kind) {
    case "install":
        return I18n.tr(lang, "Update");
    case "command":
        return I18n.tr(lang, "How to update");
    case "notes":
        return I18n.tr(lang, "Release notes");
    default:
        return "";
    }
}

function showsWhatsNew(update) {
    return update.url !== null && action(update) !== "notes";
}

function title(lang, update) {
    return I18n.tr(lang, "Headroom {version} is available", {
        version: update.version
    });
}

function started() {
    return {
        phase: "running"
    };
}

function parseEvent(line) {
    try {
        const event = JSON.parse(line);
        return isObject(event) ? event : null;
    } catch (error) {
        return null;
    }
}

function outcome(stdout, exitCode) {
    const events = String(stdout ?? "").split("\n").map(parseEvent).filter(event => event !== null);
    const done = events.find(event => event.event === "done");
    if (exitCode === 0 && done !== undefined)
        return {
            phase: "done",
            version: text(done.version),
            relogin: done.relogin === true
        };
    const failure = events.find(event => event.event === "error");
    return {
        phase: "failed",
        message: text(failure?.message),
        exitCode
    };
}

function failureText(lang, run) {
    if (run.message !== null)
        return run.message;
    if (run.exitCode === 127)
        return I18n.tr(lang, "Couldn't find the headroom command");
    return I18n.tr(lang, "The update stopped before it finished");
}

function runLine(lang, run) {
    switch (run.phase) {
    case "running":
        return I18n.tr(lang, "Updating Headroom…");
    case "done":
        if (run.relogin)
            return I18n.tr(lang, "Updated — log out and back in to finish");
        return run.version !== null ? I18n.tr(lang, "Updated to {version}", {
            version: run.version
        }) : I18n.tr(lang, "Updated");
    case "failed":
        return failureText(lang, run);
    default:
        return "";
    }
}
