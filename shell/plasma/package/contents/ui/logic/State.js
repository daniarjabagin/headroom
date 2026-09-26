.pragma library

.import "Combined.js" as Combined
.import "I18n.js" as I18n
.import "Panel.js" as Panel
.import "Parse.js" as Parse
.import "ProviderStatus.js" as ProviderStatus
.import "Recovery.js" as Recovery
.import "Refresh.js" as Refresh
.import "Settings.js" as Settings
.import "SpendState.js" as SpendState
.import "Update.js" as Update
.import "UpdateCheck.js" as UpdateCheck

const SCHEMA_VERSION = 1;
const HOUR_MS = 60 * 60 * 1000;
const LIVE_GRACE_MS = 60 * 1000;

const TONES = ["good", "warning", "critical", "neutral"];
const STATUSES = ["fresh", "stale", "refreshing", "error", "signed_out", "no_subscription"];
const WITHOUT_QUOTAS = ["signed_out", "no_subscription"];
const SEVERITIES = ["untracked", "healthy", "close", "running_out", "spent"];
const PACE_BASES = ["recent", "window", "paused"];
const BALANCE_KINDS = ["usd", "money", "count"];
const CURRENCY_CODE = /^[A-Z]{3}$/;
const OWNERS = ["cli", "headroom"];
const SOURCES = ["live", "local_log", "cache"];

class StateError extends I18n.LocalizedError {}

function parseError(raw) {
    if (!Parse.isObject(raw))
        return null;
    const kind = Parse.text(raw.kind) ?? "unknown";
    return {
        kind,
        message: Parse.text(raw.message) ?? kind
    };
}

function parsePace(raw) {
    const pace = Parse.object(raw);
    return {
        severity: Parse.oneOf(SEVERITIES, pace.severity, "untracked"),
        evenPacePercent: Parse.number(pace.even_pace_percent),
        projectedPercent: Parse.number(pace.projected_percent),
        sparePercent: Parse.number(pace.spare_percent),
        runsOutAt: Parse.timestamp(pace.runs_out_at),
        basis: Parse.oneOf(PACE_BASES, pace.basis, null),
        activeLeftSeconds: Parse.seconds(pace.active_left_seconds)
    };
}

function parseWindow(raw) {
    return {
        id: Parse.text(raw.id) ?? Parse.text(raw.label) ?? "window",
        label: Parse.text(raw.label) ?? Parse.text(raw.id) ?? "",
        usedPercent: Parse.number(raw.used_percent),
        remainingPercent: Parse.number(raw.remaining_percent),
        resetsAt: Parse.timestamp(raw.resets_at),
        hidden: raw.hidden === true,
        tone: Parse.oneOf(TONES, raw.tone, "neutral"),
        pace: parsePace(raw.pace)
    };
}

function currencyCode(value) {
    return typeof value === "string" && CURRENCY_CODE.test(value) ? value : null;
}

function parseBalance(raw) {
    const kind = Parse.oneOf(BALANCE_KINDS, raw.kind, null);
    return {
        id: Parse.text(raw.id) ?? Parse.text(raw.label) ?? "balance",
        label: Parse.text(raw.label) ?? "",
        kind,
        usdMicros: kind === "usd" ? Parse.number(raw.usd_micros) : null,
        currency: kind === "money" ? currencyCode(raw.currency) : null,
        micros: kind === "money" ? Parse.number(raw.micros) : null,
        value: kind === "count" ? Parse.number(raw.value) : null,
        unit: kind === "count" ? Parse.text(raw.unit) : null
    };
}

function parseNotice(raw) {
    return {
        tone: Parse.oneOf(TONES, raw.tone, "warning"),
        text: Parse.text(raw.text) ?? ""
    };
}

function parseAccount(raw, usage) {
    const provider = Parse.text(raw.provider) ?? "unknown";
    const usageHome = Parse.text(raw.usage_home);
    return {
        id: Parse.text(raw.id) ?? "",
        provider,
        providerName: Parse.text(raw.provider_name) ?? provider,
        label: Parse.text(raw.label),
        email: Parse.text(raw.email),
        plan: Parse.text(raw.plan),
        owner: Parse.oneOf(OWNERS, raw.owner, "cli"),
        status: Parse.oneOf(STATUSES, raw.status, "fresh"),
        error: parseError(raw.error),
        recovery: Recovery.parseRecovery(raw.recovery),
        updatedAt: Parse.timestamp(raw.updated_at),
        source: Parse.oneOf(SOURCES, raw.source, null),
        hidden: raw.hidden === true,
        collapsed: raw.collapsed === true,
        refresh: Refresh.parseRefresh(raw.refresh),
        windows: Parse.list(raw.windows).map(parseWindow),
        balances: Parse.list(raw.balances).map(parseBalance),
        notices: Parse.list(raw.notices).map(parseNotice),
        usage: usage.find(entry => entry.provider === provider && entry.usageHome === usageHome) ?? null
    };
}

function decode(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        throw new StateError(I18n.N("Unreadable state from the Headroom service: {reason}"), {
            reason: error.message
        });
    }
}

function parseState(json) {
    const raw = decode(json);
    if (!Parse.isObject(raw))
        throw new StateError(I18n.N("Unexpected state from the Headroom service"));
    if (raw.version !== SCHEMA_VERSION)
        throw new StateError(I18n.N("Headroom service speaks state version {version}, expected {expected}"), {
            version: raw.version,
            expected: SCHEMA_VERSION
        });
    const usage = Parse.list(raw.usage).map(SpendState.parseUsage);
    const display = Settings.parseDisplay(raw.display);
    const headline = Panel.parseHeadline(raw.headline);
    const accounts = Parse.list(raw.accounts).map(account => parseAccount(account, usage));
    const panelServed = Panel.served(raw);
    return {
        generatedAt: Parse.timestamp(raw.generated_at),
        nextRefreshAt: Parse.timestamp(raw.next_refresh_at),
        offline: raw.offline === true,
        lastSuccessAt: Parse.timestamp(raw.last_success_at),
        headline,
        panelItems: Panel.items(raw.panel_items, headline, accounts, display.valueMode),
        panelTone: Panel.tone(raw.panel_tone, panelServed, headline),
        supports06: panelServed,
        display,
        accounts,
        combined: Combined.parseGroups(Parse.list(raw.combined), parseWindow),
        providerStatus: ProviderStatus.parseStatuses(raw.provider_status),
        spend: SpendState.parseSpend(raw.spend, usage),
        update: Update.parseUpdate(raw.update),
        updateCheck: UpdateCheck.parseUpdateCheck(raw.update_check),
        appVersion: Parse.text(raw.app_version)
    };
}

function isStateError(error) {
    return error instanceof StateError;
}

function visibleAccounts(state) {
    return state.accounts.filter(account => !account.hidden);
}

function shownWindows(account) {
    return account.windows.filter(window => !window.hidden);
}

function withDisplay(state, patch) {
    return Object.assign({}, state, {
        display: Object.assign({}, state.display, patch)
    });
}

function withOrder(state, order) {
    const listed = order.map(id => state.accounts.find(account => account.id === id)).filter(account => account !== undefined);
    const rest = state.accounts.filter(account => !order.includes(account.id));
    return Object.assign({}, state, {
        accounts: listed.concat(rest)
    });
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

function hasQuotas(account) {
    return !WITHOUT_QUOTAS.includes(account.status);
}

function isRefreshing(state) {
    return state.accounts.some(account => account.status === "refreshing");
}

function resetsWithinHour(window, now) {
    if (window.resetsAt === null)
        return false;
    const left = window.resetsAt - now;
    return left > -LIVE_GRACE_MS && left < HOUR_MS;
}

function needsLiveClock(state, now) {
    return visibleAccounts(state).some(account => hasQuotas(account) && shownWindows(account).some(window => resetsWithinHour(window, now)));
}
