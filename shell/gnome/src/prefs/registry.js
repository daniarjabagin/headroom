import { _, fill } from '../i18n.js';

const SCHEMA_VERSION = 1;
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

function parseProvider(raw) {
    if (!isObject(raw) || typeof raw.id !== 'string' || !PROVIDER_ID.test(raw.id)) return null;
    const methods = (Array.isArray(raw.add_account) ? raw.add_account : []).map(parseMethod).filter(Boolean);
    if (methods.length === 0) return null;
    return {
        id: raw.id,
        displayName: text(raw.display_name) ?? raw.id,
        methods,
        multiAccount: raw.multi_account === true,
    };
}

function decode(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        throw new RegistryError(`Unreadable provider list from the Headroom service: ${error.message}`);
    }
}

export function parseProviders(json) {
    const raw = decode(json);
    if (!isObject(raw)) throw new RegistryError('Unexpected provider list from the Headroom service');
    if (raw.version !== SCHEMA_VERSION)
        throw new RegistryError(
            `Headroom service lists providers in version ${raw.version}, expected ${SCHEMA_VERSION}`
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
    return ['accounts', 'add', providerId, ...keyFlag, ...(label ? ['--label', label] : [])];
}

export function providerSummary(provider) {
    return provider.methods.map(methodSummary).join(' · ');
}
