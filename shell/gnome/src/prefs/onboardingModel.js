import { isSignedOut } from '../accountStatus.js';
import { _ } from '../i18n.js';
import { accountTitle, showsName } from '../providers.js';

function signedInSubtitle(account) {
    return [_('Signed in'), account.plan, account.email].filter(Boolean).join(' · ');
}

function accountEntry(account, accounts) {
    const base = {
        key: account.id,
        provider: account.provider,
        title: accountTitle(account, showsName(account, accounts)),
    };
    if (isSignedOut(account)) return { ...base, kind: 'signed_out', subtitle: _('Found, not signed in'), account };
    return {
        ...base,
        kind: 'account',
        subtitle: signedInSubtitle(account),
        accountId: account.id,
        shown: !account.hidden,
    };
}

function cliProgram(provider) {
    return provider.methods.find(method => method.kind === 'cli_login' && method.program)?.program ?? null;
}

function providerEntry(provider, installed) {
    const base = { key: provider.id, provider: provider.id, title: provider.displayName, registry: provider };
    if (installed) return { ...base, kind: 'found', subtitle: _('Found, not signed in') };
    return { ...base, kind: 'missing', subtitle: _('Not installed') };
}

export function onboardingEntries(accounts, providers, isInstalled) {
    const withAccounts = new Set(accounts.map(account => account.provider));
    const candidates = providers
        .filter(provider => !withAccounts.has(provider.id))
        .map(provider => [provider, cliProgram(provider)])
        .filter(([, program]) => program !== null)
        .map(([provider, program]) => providerEntry(provider, isInstalled(program)));
    return [
        ...accounts.map(account => accountEntry(account, accounts)),
        ...candidates.filter(entry => entry.kind === 'found'),
        ...candidates.filter(entry => entry.kind === 'missing'),
    ];
}
