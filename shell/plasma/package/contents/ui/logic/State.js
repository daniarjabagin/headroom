.pragma library

const SCHEMA_VERSION = 1;

const TONES = ["good", "warning", "critical", "neutral"];
const STATUSES = ["fresh", "stale", "refreshing", "error", "signed_out"];
const SEVERITIES = ["untracked", "healthy", "close", "running_out", "spent"];
const BALANCE_KINDS = ["usd", "count"];

class StateError extends Error {}

function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function text(value) {
    return typeof value === "string" && value.length > 0 ? value : null;
}

function number(value) {
    return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function count(value) {
    return number(value) ?? 0;
}

function list(value) {
    return Array.isArray(value) ? value.filter(isObject) : [];
}

function timestamp(value) {
    const ms = typeof value === "string" ? Date.parse(value) : NaN;
    return Number.isNaN(ms) ? null : new Date(ms);
}

function oneOf(allowed, value, fallback) {
    return allowed.includes(value) ? value : fallback;
}

function parseError(raw) {
    if (!isObject(raw))
        return null;
    const kind = text(raw.kind) ?? "unknown";
    return {
        kind,
        message: text(raw.message) ?? kind
    };
}

function parsePace(raw) {
    const pace = isObject(raw) ? raw : {};
    return {
        severity: oneOf(SEVERITIES, pace.severity, "untracked"),
        evenPacePercent: number(pace.even_pace_percent),
        projectedPercent: number(pace.projected_percent),
        sparePercent: number(pace.spare_percent),
        runsOutAt: timestamp(pace.runs_out_at)
    };
}

function parseWindow(raw) {
    return {
        id: text(raw.id) ?? text(raw.label) ?? "window",
        label: text(raw.label) ?? text(raw.id) ?? "",
        remainingPercent: number(raw.remaining_percent),
        resetsAt: timestamp(raw.resets_at),
        tone: oneOf(TONES, raw.tone, "neutral"),
        pace: parsePace(raw.pace)
    };
}

function parseBalance(raw) {
    const kind = oneOf(BALANCE_KINDS, raw.kind, null);
    return {
        id: text(raw.id) ?? text(raw.label) ?? "balance",
        label: text(raw.label) ?? "",
        kind,
        usdMicros: kind === "usd" ? number(raw.usd_micros) : null,
        value: kind === "count" ? number(raw.value) : null,
        unit: kind === "count" ? text(raw.unit) : null
    };
}

function parseNotice(raw) {
    return {
        tone: oneOf(TONES, raw.tone, "warning"),
        text: text(raw.text) ?? ""
    };
}

function parseTokens(raw) {
    const tokens = isObject(raw) ? raw : {};
    return {
        input: count(tokens.input),
        cacheRead: count(tokens.cache_read),
        cacheWrite: count(tokens.cache_write),
        output: count(tokens.output),
        reasoning: count(tokens.reasoning),
        total: count(tokens.total)
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
        unpricedModels: Array.isArray(totals.unpriced_models) ? totals.unpriced_models.filter(text) : []
    };
}

function parseDay(raw) {
    return {
        date: text(raw.date) ?? "",
        totalTokens: count(raw.total_tokens),
        costMicros: count(raw.cost_usd_micros),
        partial: raw.partial === true
    };
}

function parseUsage(raw) {
    return {
        provider: text(raw.provider) ?? "unknown",
        usageHome: text(raw.usage_home),
        today: parseTotals(raw.today),
        yesterday: parseTotals(raw.yesterday),
        last30Days: parseTotals(raw.last_30_days),
        daily: list(raw.daily).map(parseDay)
    };
}

function parseAccount(raw, usage) {
    const provider = text(raw.provider) ?? "unknown";
    const usageHome = text(raw.usage_home);
    return {
        id: text(raw.id) ?? "",
        provider,
        label: text(raw.label),
        email: text(raw.email),
        plan: text(raw.plan),
        status: oneOf(STATUSES, raw.status, "fresh"),
        error: parseError(raw.error),
        updatedAt: timestamp(raw.updated_at),
        hidden: raw.hidden === true,
        windows: list(raw.windows).map(parseWindow),
        balances: list(raw.balances).map(parseBalance),
        notices: list(raw.notices).map(parseNotice),
        usage: usage.find(entry => entry.provider === provider && entry.usageHome === usageHome) ?? null
    };
}

function parseHeadline(raw) {
    if (!isObject(raw))
        return null;
    const remainingPercent = number(raw.remaining_percent);
    if (remainingPercent === null)
        return null;
    return {
        accountId: text(raw.account_id),
        windowId: text(raw.window),
        remainingPercent,
        tone: oneOf(TONES, raw.tone, "neutral")
    };
}

function parseProviderSpend(raw) {
    return {
        provider: text(raw.provider) ?? "unknown",
        costMicros: count(raw.cost_usd_micros),
        totalTokens: count(raw.total_tokens),
        partial: raw.partial === true
    };
}

function parsePeriod(raw) {
    const period = isObject(raw) ? raw : {};
    return {
        costMicros: count(period.cost_usd_micros),
        totalTokens: count(period.total_tokens),
        partial: period.partial === true,
        providers: list(period.by_provider).map(parseProviderSpend)
    };
}

function parseSpend(raw, usage) {
    if (usage.length === 0 || !isObject(raw))
        return null;
    return {
        today: parsePeriod(raw.today),
        yesterday: parsePeriod(raw.yesterday),
        last30Days: parsePeriod(raw.last_30_days)
    };
}

function decode(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        throw new StateError(`Unreadable state from the Headroom service: ${error.message}`);
    }
}

function parseState(json) {
    const raw = decode(json);
    if (!isObject(raw))
        throw new StateError("Unexpected state from the Headroom service");
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
        spend: parseSpend(raw.spend, usage)
    };
}

function isStateError(error) {
    return error instanceof StateError;
}

function visibleAccounts(state) {
    return state.accounts.filter(account => !account.hidden);
}

function showsName(account, accounts) {
    return accounts.filter(other => other.provider === account.provider).length > 1;
}

function headlineAccount(state) {
    if (state.headline === null)
        return null;
    return state.accounts.find(account => account.id === state.headline.accountId) ?? null;
}

function headlineWindow(state) {
    const account = headlineAccount(state);
    if (account === null)
        return null;
    return account.windows.find(window => window.id === state.headline.windowId) ?? null;
}

function isHeadlineStale(state) {
    if (state.offline)
        return true;
    return headlineAccount(state)?.status === "stale";
}

function isRefreshing(state) {
    return state.accounts.some(account => account.status === "refreshing");
}
