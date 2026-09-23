const SCHEMA_VERSION = 1;

const TONES = new Set(['good', 'warning', 'critical', 'neutral']);
const STATUSES = new Set(['fresh', 'stale', 'refreshing', 'error', 'signed_out']);
const SEVERITIES = new Set(['untracked', 'healthy', 'close', 'running_out', 'spent']);

export class StateError extends Error {}

function isObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function text(value) {
    return typeof value === 'string' && value.length > 0 ? value : null;
}

function number(value) {
    return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function count(value) {
    return number(value) ?? 0;
}

function list(value) {
    return Array.isArray(value) ? value.filter(isObject) : [];
}

function timestamp(value) {
    const ms = typeof value === 'string' ? Date.parse(value) : NaN;
    return Number.isNaN(ms) ? null : new Date(ms);
}

function oneOf(allowed, value, fallback) {
    return allowed.has(value) ? value : fallback;
}

function errorMessage(value) {
    if (isObject(value)) return text(value.message) ?? text(value.kind);
    return text(value);
}

function parsePace(raw) {
    const pace = isObject(raw) ? raw : {};
    return {
        severity: oneOf(SEVERITIES, pace.severity, 'untracked'),
        evenPacePercent: number(pace.even_pace_percent),
        projectedPercent: number(pace.projected_percent),
        runsOutAt: timestamp(pace.runs_out_at),
    };
}

function parseWindow(raw) {
    return {
        id: text(raw.id) ?? text(raw.label) ?? 'window',
        label: text(raw.label) ?? text(raw.id) ?? '',
        remainingPercent: number(raw.remaining_percent),
        resetsAt: timestamp(raw.resets_at),
        tone: oneOf(TONES, raw.tone, 'neutral'),
        pace: parsePace(raw.pace),
    };
}

function parseBalance(raw) {
    return {
        id: text(raw.id) ?? text(raw.label) ?? 'balance',
        label: text(raw.label) ?? '',
        usdMicros: number(raw.usd_micros),
        value: number(raw.value),
        unit: text(raw.unit),
    };
}

function parseNotice(raw) {
    return { tone: oneOf(TONES, raw.tone, 'warning'), text: text(raw.text) ?? '' };
}

function parseTotals(raw) {
    const totals = isObject(raw) ? raw : {};
    const tokens = isObject(totals.tokens) ? totals.tokens : {};
    return {
        costMicros: count(totals.cost_usd_micros),
        totalTokens: count(tokens.total),
        partial: totals.partial === true,
    };
}

function parseDay(raw) {
    return {
        date: text(raw.date) ?? '',
        totalTokens: count(raw.total_tokens),
        costMicros: count(raw.cost_usd_micros),
    };
}

function parseUsage(raw) {
    return {
        provider: text(raw.provider) ?? 'unknown',
        usageHome: text(raw.usage_home),
        today: parseTotals(raw.today),
        yesterday: parseTotals(raw.yesterday),
        month: parseTotals(raw.last_30_days),
        daily: list(raw.daily).map(parseDay),
    };
}

function parseAccount(raw, usage) {
    const provider = text(raw.provider) ?? 'unknown';
    const usageHome = text(raw.usage_home);
    return {
        id: text(raw.id) ?? '',
        provider,
        label: text(raw.label),
        email: text(raw.email),
        plan: text(raw.plan),
        status: oneOf(STATUSES, raw.status, 'fresh'),
        error: errorMessage(raw.error),
        updatedAt: timestamp(raw.updated_at),
        hidden: raw.hidden === true,
        windows: list(raw.windows).map(parseWindow),
        balances: list(raw.balances).map(parseBalance),
        notices: list(raw.notices).map(parseNotice),
        usage: usage.find(u => u.provider === provider && u.usageHome === usageHome) ?? null,
    };
}

function parseHeadline(raw) {
    if (!isObject(raw)) return null;
    const remainingPercent = number(raw.remaining_percent);
    if (remainingPercent === null) return null;
    return {
        accountId: text(raw.account_id),
        windowId: text(raw.window),
        remainingPercent,
        tone: oneOf(TONES, raw.tone, 'neutral'),
    };
}

function sumPeriod(usage, period) {
    const slices = usage
        .map(u => ({ provider: u.provider, usageHome: u.usageHome, ...u[period] }))
        .filter(slice => slice.costMicros > 0 || slice.totalTokens > 0);
    return {
        slices,
        costMicros: slices.reduce((sum, slice) => sum + slice.costMicros, 0),
        totalTokens: slices.reduce((sum, slice) => sum + slice.totalTokens, 0),
        partial: slices.some(slice => slice.partial),
    };
}

function parseSpend(usage) {
    if (usage.length === 0) return null;
    return {
        today: sumPeriod(usage, 'today'),
        yesterday: sumPeriod(usage, 'yesterday'),
        month: sumPeriod(usage, 'month'),
    };
}

function decode(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        throw new StateError(`Unreadable state from the Headroom service: ${error.message}`);
    }
}

export function parseState(json) {
    const raw = decode(json);
    if (!isObject(raw)) throw new StateError('Unexpected state from the Headroom service');
    if (raw.version !== SCHEMA_VERSION)
        throw new StateError(`Headroom service speaks state version ${raw.version}, expected ${SCHEMA_VERSION}`);
    const usage = list(raw.usage).map(parseUsage);
    return {
        generatedAt: timestamp(raw.generated_at),
        nextRefreshAt: timestamp(raw.next_refresh_at),
        offline: raw.offline === true,
        lastSuccessAt: timestamp(raw.last_success_at),
        headline: parseHeadline(raw.headline),
        accounts: list(raw.accounts).map(account => parseAccount(account, usage)),
        spend: parseSpend(usage),
    };
}
