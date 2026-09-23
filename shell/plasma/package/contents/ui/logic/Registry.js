.pragma library

.import "I18n.js" as I18n

const SCHEMA_VERSION = 1;
const PROVIDER_ID = /^[a-z0-9][a-z0-9_-]*$/;
const METHOD_KINDS = ["cli_login", "api_key", "auto_detect"];
const WEB_URL = /^https?:\/\/\S+$/;

class RegistryError extends I18n.LocalizedError {}

function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function text(value) {
    return typeof value === "string" && value.length > 0 ? value : null;
}

function webUrl(value) {
    const url = text(value);
    return url !== null && WEB_URL.test(url) ? url : null;
}

function parseMethod(raw) {
    if (!isObject(raw) || !METHOD_KINDS.includes(raw.kind))
        return null;
    return {
        kind: raw.kind,
        program: text(raw.program),
        label: text(raw.label),
        consoleUrl: webUrl(raw.console_url),
        hint: text(raw.hint),
        reason: text(raw.reason)
    };
}

function parseProvider(raw) {
    const id = isObject(raw) ? text(raw.id) : null;
    if (id === null || !PROVIDER_ID.test(id) || !Array.isArray(raw.add_account))
        return null;
    const method = parseMethod(raw.add_account[0]);
    if (method === null)
        return null;
    return {
        id,
        name: text(raw.display_name) ?? id,
        method,
        multiAccount: raw.multi_account === true,
        localUsage: raw.local_usage === true
    };
}

function decode(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        throw new RegistryError(I18n.N("Unreadable provider list from the Headroom service: {reason}"), {
            reason: error.message
        });
    }
}

function parseRegistry(json) {
    const raw = decode(json);
    if (!isObject(raw) || !Array.isArray(raw.providers))
        throw new RegistryError(I18n.N("Unexpected provider list from the Headroom service"));
    if (raw.version !== SCHEMA_VERSION)
        throw new RegistryError(I18n.N("Headroom service speaks provider list version {version}, expected {expected}"), {
            version: raw.version,
            expected: SCHEMA_VERSION
        });
    return raw.providers.map(parseProvider).filter(provider => provider !== null);
}

function isRegistryError(error) {
    return error instanceof RegistryError;
}

function findProvider(providers, id) {
    return providers.find(provider => provider.id === id) ?? null;
}

function keyHint(method) {
    return [method.label, method.hint].filter(part => part !== null).join(": ");
}

function methodHint(lang, provider) {
    const method = provider.method;
    if (method.kind === "cli_login")
        return I18n.tr(lang, "Signs in with the {program} CLI in a terminal", {
            program: method.program ?? provider.id
        });
    if (method.kind === "api_key")
        return I18n.tr(lang, "Asks for an API key in a terminal");
    return method.reason ?? I18n.tr(lang, "Found automatically");
}
