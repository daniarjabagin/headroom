import { parseDisplay } from './settings.js';
import { parseUpdate } from './update.js';

const SCHEMA_VERSION = 1;

const TONES = new Set(['good', 'warning', 'critical', 'neutral']);
const STATUSES = new Set(['fresh', 'stale', 'refreshing', 'error', 'signed_out', 'no_subscription']);
const SEVERITIES = new Set(['untracked', 'healthy', 'close', 'running_out', 'spent']);
const BALANCE_KINDS = new Set(['usd', 'count']);
const OWNERS = new Set(['cli', 'headroom']);

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

function parseError(raw) {
    if (!isObject(raw)) return null;
    const kind = text(raw.kind) ?? 'unknown';
    return { kind, message: text(raw.message) ?? kind };
}

function parsePace(raw) {
    const pace = isObject(raw) ? raw : {};
    return {
        severity: oneOf(SEVERITIES, pace.severity, 'untracked'),
        evenPacePercent: number(pace.even_pace_percent),
        projectedPercent: number(pace.projected_percent),
        sparePercent: number(pace.spare_percent),
        runsOutAt: timestamp(pace.runs_out_at),
    };
}

function parseWindow(raw) {
    return {
        id: text(raw.id) ?? text(raw.label) ?? 'window',
        label: text(raw.label) ?? text(raw.id) ?? '',
        usedPercent: number(raw.used_percent),
        remainingPercent: number(raw.remaining_percent),
        resetsAt: timestamp(raw.resets_at),
        hidden: raw.hidden === true,
        tone: oneOf(TONES, raw.tone, 'neutral'),
        pace: parsePace(raw.pace),
    };
}

function parseBalance(raw) {
    const kind = oneOf(BALANCE_KINDS, raw.kind, null);
    return {
        id: text(raw.id) ?? text(raw.label) ?? 'balance',
        label: text(raw.label) ?? '',
        kind,
        usdMicros: kind === 'usd' ? number(raw.usd_micros) : null,
        value: kind === 'count' ? number(raw.value) : null,
        unit: kind === 'count' ? text(raw.unit) : null,
    };
}

function parseNotice(raw) {
    return { tone: oneOf(TONES, raw.tone, 'warning'), text: text(raw.text) ?? '' };
}

function parseTokens(raw) {
    const tokens = isObject(raw) ? raw : {};
    return {
        input: count(tokens.input),
        cacheRead: count(tokens.cache_read),
        cacheWrite: count(tokens.cache_write),
        output: count(tokens.output),
        reasoning: count(tokens.reasoning),
        total: count(tokens.total),
    };
}

function parseModel(raw) {
    return {
        model: text(raw.model) ?? 'unknown',
        totalTokens: count(raw.total_tokens),
        costMicros: count(raw.cost_usd_micros),
        partial: raw.partial === true,
    };
}

function parseModels(raw) {
    return list(raw).map(parseModel);
}

function parseModelsOther(raw) {
    if (!isObject(raw)) return null;
    return {
        count: count(raw.count),
        totalTokens: count(raw.total_tokens),
        costMicros: count(raw.cost_usd_micros),
        partial: raw.partial === true,
    };
}

function parseTotals(raw) {
    const totals = isObject(raw) ? raw : {};
    const tokens = parseTokens(totals.tokens);
    return {
        tokens,
        costMicros: count(totals.cost_usd_micros),
        totalTokens: tokens.total,
        partial: totals.partial === true,
        unpricedTokens: count(totals.unpriced_tokens),
        unpricedModels: Array.isArray(totals.unpriced_models) ? totals.unpriced_models.filter(text) : [],
        models: parseModels(totals.models),
        modelsOther: parseModelsOther(totals.models_other),
    };
}

function parseDay(raw) {
    return {
        date: text(raw.date) ?? '',
        totalTokens: count(raw.total_tokens),
        costMicros: count(raw.cost_usd_micros),
        partial: raw.partial === true,
    };
}

function providerOf(raw) {
    const provider = text(raw.provider) ?? 'unknown';
    return { provider, providerName: text(raw.provider_name) ?? provider };
}

function parseUsage(raw) {
    return {
        ...providerOf(raw),
        usageHome: text(raw.usage_home),
        today: parseTotals(raw.today),
        yesterday: parseTotals(raw.yesterday),
        last30Days: parseTotals(raw.last_30_days),
        daily: list(raw.daily).map(parseDay),
    };
}

function parseAccount(raw, usage) {
    const { provider, providerName } = providerOf(raw);
    const usageHome = text(raw.usage_home);
    return {
        id: text(raw.id) ?? '',
        provider,
        providerName,
        label: text(raw.label),
        email: text(raw.email),
        plan: text(raw.plan),
        owner: oneOf(OWNERS, raw.owner, 'cli'),
        status: oneOf(STATUSES, raw.status, 'fresh'),
        error: parseError(raw.error),
        updatedAt: timestamp(raw.updated_at),
        hidden: raw.hidden === true,
        windows: list(raw.windows).map(parseWindow),
        balances: list(raw.balances).map(parseBalance),
        notices: list(raw.notices).map(parseNotice),
        usage: usage.find(u => u.provider === provider && u.usageHome === usageHome) ?? null,
    };
}

function parseSegment(raw) {
    return {
        accountId: text(raw.account_id) ?? '',
        label: text(raw.label),
        remainingPercent: number(raw.remaining_percent),
        usedPercent: number(raw.used_percent),
        resetsAt: timestamp(raw.resets_at),
        tone: oneOf(TONES, raw.tone, 'neutral'),
    };
}

function parseCombinedWindow(raw) {
    return {
        ...parseWindow(raw),
        capacityPercent: number(raw.capacity_percent) ?? 100,
        segments: list(raw.segments).map(parseSegment),
    };
}

function parseMember(raw) {
    return { accountId: text(raw.account_id) ?? '', label: text(raw.label), plan: text(raw.plan) };
}

function parseGroup(raw) {
    return {
        ...providerOf(raw),
        accountIds: Array.isArray(raw.account_ids) ? raw.account_ids.filter(text) : [],
        accounts: list(raw.accounts).map(parseMember),
        windows: list(raw.windows).map(parseCombinedWindow),
    };
}

function parseHeadline(raw) {
    if (!isObject(raw)) return null;
    const remainingPercent = number(raw.remaining_percent);
    if (remainingPercent === null) return null;
    return {
        accountId: text(raw.account_id),
        windowId: text(raw.window),
        provider: text(raw.provider),
        providerName: text(raw.provider_name) ?? text(raw.provider),
        accountLabel: text(raw.account_label),
        windowLabel: text(raw.window_label),
        usedPercent: number(raw.used_percent),
        remainingPercent,
        tone: oneOf(TONES, raw.tone, 'neutral'),
        combined: raw.combined === true,
        accountCount: number(raw.account_count),
    };
}

function parseProviderSpend(raw) {
    return {
        ...providerOf(raw),
        costMicros: count(raw.cost_usd_micros),
        totalTokens: count(raw.total_tokens),
        partial: raw.partial === true,
        models: parseModels(raw.models),
        modelsOther: parseModelsOther(raw.models_other),
    };
}

function parsePeriod(raw) {
    const period = isObject(raw) ? raw : {};
    return {
        costMicros: count(period.cost_usd_micros),
        totalTokens: count(period.total_tokens),
        partial: period.partial === true,
        providers: list(period.by_provider).map(parseProviderSpend),
    };
}

function parseSpend(raw, usage) {
    if (usage.length === 0 || !isObject(raw)) return null;
    return {
        today: parsePeriod(raw.today),
        yesterday: parsePeriod(raw.yesterday),
        last30Days: parsePeriod(raw.last_30_days),
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
        display: parseDisplay(raw.display),
        accounts: list(raw.accounts).map(account => parseAccount(account, usage)),
        combined: list(raw.combined)
            .map(parseGroup)
            .filter(group => group.accountIds.length > 0),
        spend: parseSpend(raw.spend, usage),
        update: parseUpdate(raw.update),
    };
}

export function isRefreshing(state) {
    return state.accounts.some(account => account.status === 'refreshing');
}
