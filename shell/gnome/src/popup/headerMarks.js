import { _ } from '../i18n.js';
import { statusTitle } from './statusTexts.js';

function failedOffline(ctx, account) {
    return ctx.offline && account.error?.kind === 'network';
}

function accountStatusKind(ctx, account) {
    if (account.status === 'refreshing') return 'refreshing';
    if (account.status === 'stale' || (account.status === 'error' && failedOffline(ctx, account))) return 'outdated';
    if (account.status === 'error') return 'error';
    return null;
}

export function headerStatusKind(ctx, account, incident) {
    const kind = accountStatusKind(ctx, account);
    return kind === 'error' && incident !== null ? null : kind;
}

export function incidentTip(ctx, incident, account) {
    const title = statusTitle(incident);
    if (accountStatusKind(ctx, account) !== 'error') return title;
    return `${title}\n${account.error?.message ?? _('Refresh failed')}`;
}
