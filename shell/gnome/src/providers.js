const PROVIDER_ID = /^[a-z0-9_-]+$/;
const SERIES = new Set([
    'codex',
    'claude',
    'opencode',
    'openrouter',
    'zai',
    'kimi',
    'minimax',
    'grok',
    'cline',
    'devin',
    'copilot',
    'cursor',
    'antigravity',
    'ollama',
]);
const FALLBACK_SERIES = 4;

export const GENERIC_ICON = { file: 'provider-symbolic.svg', tinted: true };

function stableHash(value) {
    let hash = 0;
    for (const char of value) hash = (hash * 31 + char.codePointAt(0)) >>> 0;
    return hash;
}

export function seriesKey(id) {
    return SERIES.has(id) ? id : `other-${stableHash(String(id)) % FALLBACK_SERIES}`;
}

export function iconCandidates(id) {
    if (typeof id !== 'string' || !PROVIDER_ID.test(id)) return [];
    return [
        { file: `${id}.svg`, tinted: false },
        { file: `${id}-symbolic.svg`, tinted: true },
    ];
}

export function showsName(account, accounts) {
    return accounts.filter(other => other.provider === account.provider).length > 1;
}

export function accountTitle(account, showName) {
    const name = account.providerName;
    if (!showName) return name;
    const who = account.label ?? account.email;
    return who ? `${name}: ${who}` : name;
}

export function accountName(account) {
    return account.label ?? account.email ?? account.providerName;
}
