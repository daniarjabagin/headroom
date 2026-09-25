import { httpsUrl, list, oneOf, text, timestamp } from './fields.js';

const INDICATORS = new Set(['none', 'minor', 'major', 'critical', 'maintenance']);
const INDICATOR_TONES = {
    none: 'neutral',
    minor: 'warning',
    maintenance: 'warning',
    major: 'critical',
    critical: 'critical',
};
const STATUS_TONES = new Set(['neutral', 'warning', 'critical']);

function parseEntry(raw) {
    const provider = text(raw.provider);
    const indicator = oneOf(INDICATORS, raw.indicator, null);
    if (provider === null || indicator === null) return null;
    return {
        provider,
        indicator,
        tone: oneOf(STATUS_TONES, raw.tone, INDICATOR_TONES[indicator]),
        title: text(raw.title),
        stage: text(raw.stage),
        startedAt: timestamp(raw.started_at),
        url: httpsUrl(raw.url),
    };
}

export function parseProviderStatus(raw) {
    return list(raw).map(parseEntry).filter(Boolean);
}

export function providerIncident(statuses, provider) {
    return statuses.find(entry => entry.provider === provider && entry.indicator !== 'none') ?? null;
}
