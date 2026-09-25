import { _, fill } from '../i18n.js';

const SCHEMA_VERSION = 1;
export const LABEL_MAX_CHARS = 64;
const PROVIDER_ID = /^[a-z0-9_-]+$/;

export class RegistryError extends Error {}

function isObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function text(value) {
    return typeof value === 'string' && value.trim().length > 0 ? value.trim() : null;
}

function httpsUrl(value) {
    const url = text(value);
    return url && /^https:\/\/\S+$/.test(url) ? url : null;
}

function parseMethod(raw) {
    if (!isObject(raw)) return null;
    if (raw.kind === 'cli_login') return { kind: 'cli_login', program: text(raw.program) };
    if (raw.kind === 'api_key')
        return { kind: 'api_key', label: text(raw.label), consoleUrl: httpsUrl(raw.console_url), hint: text(raw.hint) };
    if (raw.kind === 'auto_detect') return { kind: 'auto_detect', reason: text(raw.reason) };
    return null;
}

function parseLinks(raw) {
    const links = isObject(raw) ? raw : {};
    return { status: httpsUrl(links.status), dashboard: httpsUrl(links.dashboard), usage: httpsUrl(links.usage) };
}

function parseProvider(raw) {
    if (!isObject(raw) || typeof raw.id !== 'string' || !PROVIDER_ID.test(raw.id)) return null;
    const methods = (Array.isArray(raw.add_account) ? raw.add_account : []).map(parseMethod).filter(Boolean);
    if (methods.length === 0) return null;
    return {
        id: raw.id,
        displayName: text(raw.display_name) ?? raw.id,
        methods,
        multiAccount: raw.multi_account === true,
        links: parseLinks(raw.links),
    };
}

function decode(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        throw new RegistryError(
            fill(_('Unreadable provider list from the Headroom service: {reason}'), { reason: error.message })
        );
    }
}

export function parseProviders(json) {
    const raw = decode(json);
    if (!isObject(raw)) throw new RegistryError(_('Unexpected provider list from the Headroom service'));
    if (raw.version !== SCHEMA_VERSION)
        throw new RegistryError(
            fill(_('Headroom service lists providers in version {actual}, expected {expected}'), {
                actual: raw.version,
                expected: SCHEMA_VERSION,
            })
        );
    const providers = (Array.isArray(raw.providers) ? raw.providers : []).map(parseProvider).filter(Boolean);
    return providers.filter((provider, index) => providers.findIndex(other => other.id === provider.id) === index);
}

export function methodSummary(method) {
    if (method.kind === 'cli_login')
        return method.program ? fill(_('Sign in with {program}'), { program: method.program }) : _('Sign in');
    if (method.kind === 'api_key') return _('API key');
    return _('Detected automatically');
}

export function addAccountArgs(providerId, method, label) {
    const keyFlag = method.kind === 'api_key' ? ['--api-key-stdin'] : [];
    const trimmed = label.trim();
    return ['accounts', 'add', providerId, ...keyFlag, ...(trimmed ? [`--label=${trimmed}`] : [])];
}

export function loginArgs(accountId, method) {
    return ['accounts', 'login', accountId, ...(method.kind === 'api_key' ? ['--api-key-stdin'] : [])];
}

export function loginMethod(provider) {
    return (
        provider.methods.find(method => method.kind === 'cli_login') ??
        provider.methods.find(method => method.kind === 'api_key') ??
        null
    );
}

export function signInTarget(account, providers) {
    if (account.recovery?.action !== 'sign_in') return null;
    const provider = (providers ?? []).find(entry => entry.id === account.provider);
    if (!provider || !loginMethod(provider)) return null;
    return { provider, loginId: account.recovery.accountId ?? account.id };
}

export function providerSummary(provider) {
    return provider.methods.map(methodSummary).join(' · ');
}
