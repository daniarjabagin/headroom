import { integer, isObject, number, text } from './fields.js';

export class DiagnosticsError extends Error {}

function decode(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        throw new DiagnosticsError(`Unreadable diagnostics from the Headroom service: ${error.message}`);
    }
}

function parseProviderCount(raw) {
    return {
        provider: text(raw.provider),
        accounts: integer(raw.accounts) ?? 0,
        usageHomes: integer(raw.usage_homes) ?? 0,
    };
}

export function parseDiagnostics(json) {
    const raw = decode(json);
    const report = isObject(raw) ? text(raw.text) : null;
    if (report === null) throw new DiagnosticsError('Unexpected diagnostics from the Headroom service');
    return {
        appVersion: text(raw.app_version),
        os: text(raw.os),
        desktop: text(raw.desktop),
        uptimeSecs: number(raw.uptime_secs),
        transports: Array.isArray(raw.transports) ? raw.transports.filter(text) : [],
        logLevel: text(raw.log_level),
        logLevelSource: text(raw.log_level_source),
        logFile: text(raw.log_file),
        providers: (Array.isArray(raw.providers) ? raw.providers.filter(isObject) : []).map(parseProviderCount),
        text: report,
    };
}
