import { integer, isObject, list, number, oneOf, providerOf, text, timestamp, TONES } from './fields.js';
import { parseHeadline, parsePanelItems, parsePanelTone, reportsPanelItems } from './panelItems.js';
import { parseProviderStatus } from './providerStatus.js';
import { parseDisplay } from './settings.js';
import { parseSpend, parseUsage } from './spendPayload.js';
import { parseUpdate } from './update.js';
import { parseUpdateCheck } from './updateCheck.js';

const SCHEMA_VERSION = 1;

const STATUSES = new Set(['fresh', 'stale', 'refreshing', 'error', 'signed_out', 'no_subscription']);
const SEVERITIES = new Set(['untracked', 'healthy', 'close', 'running_out', 'spent']);
const BALANCE_KINDS = new Set(['usd', 'money', 'count']);
const CURRENCY_CODE = /^[A-Z]{3}$/;
const OWNERS = new Set(['cli', 'headroom']);
const REFRESH_MODES = new Set(['live', 'idle']);
const REFRESH_REASONS = new Set(['hold', 'backoff', 'activity', 'schedule']);
const MIN_REFRESH_SECS = 60;

export class StateError extends Error {}

function parseError(raw) {
    if (!isObject(raw)) return null;
    const kind = text(raw.kind) ?? 'unknown';
    return { kind, message: text(raw.message) ?? kind };
}

function parseRecovery(raw) {
    if (!isObject(raw)) return null;
    if (raw.action === 'retry') return { action: 'retry' };
    if (raw.action === 'sign_in') return { action: 'sign_in', accountId: text(raw.account_id) };
    const command = text(raw.command);
    if (raw.action === 'cli_login' && command) return { action: 'cli_login', command };
    return null;
}

function refreshInterval(value) {
    const secs = integer(value);
    return secs !== null && secs >= MIN_REFRESH_SECS ? secs : null;
}

function parseRefresh(raw) {
    if (!isObject(raw)) return null;
    const mode = oneOf(REFRESH_MODES, raw.mode, 'idle');
    return {
        mode,
        intervalSecs: refreshInterval(raw.interval_secs),
        nextAt: timestamp(raw.next_at),
        reason: oneOf(REFRESH_REASONS, raw.reason, mode === 'live' ? 'activity' : 'schedule'),
    };
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

function currencyCode(value) {
    return typeof value === 'string' && CURRENCY_CODE.test(value) ? value : null;
}

function parseBalance(raw) {
    const kind = oneOf(BALANCE_KINDS, raw.kind, null);
    return {
        id: text(raw.id) ?? text(raw.label) ?? 'balance',
        label: text(raw.label) ?? '',
        kind,
        usdMicros: kind === 'usd' ? number(raw.usd_micros) : null,
        currency: kind === 'money' ? currencyCode(raw.currency) : null,
        micros: kind === 'money' ? number(raw.micros) : null,
        value: kind === 'count' ? number(raw.value) : null,
        unit: kind === 'count' ? text(raw.unit) : null,
    };
}

function parseNotice(raw) {
    return { tone: oneOf(TONES, raw.tone, 'warning'), text: text(raw.text) ?? '' };
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
        recovery: parseRecovery(raw.recovery),
        refresh: parseRefresh(raw.refresh),
        collapsed: raw.collapsed === true,
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
        collapsed: raw.collapsed === true,
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
    const headline = parseHeadline(raw.headline);
    const display = parseDisplay(raw.display);
    return {
        appVersion: text(raw.app_version),
        generatedAt: timestamp(raw.generated_at),
        nextRefreshAt: timestamp(raw.next_refresh_at),
        offline: raw.offline === true,
        lastSuccessAt: timestamp(raw.last_success_at),
        headline,
        display,
        panelItems: parsePanelItems(raw, headline, display),
        panelTone: parsePanelTone(raw, headline),
        reportsPanelItems: reportsPanelItems(raw),
        accounts: list(raw.accounts).map(account => parseAccount(account, usage)),
        combined: list(raw.combined)
            .map(parseGroup)
            .filter(group => group.accountIds.length > 0),
        spend: parseSpend(raw.spend, usage),
        update: parseUpdate(raw.update),
        updateCheck: parseUpdateCheck(raw.update_check),
        providerStatus: parseProviderStatus(raw.provider_status),
    };
}

export function isRefreshing(state) {
    return state.accounts.some(account => account.status === 'refreshing');
}
