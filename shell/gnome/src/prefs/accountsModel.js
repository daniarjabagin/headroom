import { accountNotice } from '../accountStatus.js';
import { _, fill } from '../i18n.js';
import { moveItem } from '../order.js';
import { accountName } from '../providers.js';
import { isStarred, isWindowHidden } from '../settings.js';

const TONE_RANK = ['neutral', 'good', 'warning', 'critical'];

export function sidebarTitle(account, accounts) {
    const sameProvider = accounts.filter(other => other.provider === account.provider).length;
    if (sameProvider < 2 && !account.label) return account.providerName;
    return `${account.providerName} · ${accountName(account)}`;
}

export function sidebarSubtitle(account, title) {
    const email = account.email && !title.includes(account.email) ? account.email : null;
    return [account.plan, email].filter(Boolean).join(' · ');
}

export function detailSubtitle(account) {
    const parts = [account.providerName, account.plan, account.label ? account.email : null];
    if (account.owner === 'headroom') parts.push(_('added in Headroom'));
    return parts.filter(Boolean).join(' · ');
}

function worstTone(tones) {
    return tones.reduce(
        (worst, tone) => (TONE_RANK.indexOf(tone) > TONE_RANK.indexOf(worst) ? tone : worst),
        'neutral'
    );
}

export function accountMark(account, display, offline) {
    const notice = accountNotice(account, offline);
    if (notice !== null) return { kind: 'notice', notice };
    const shown = account.windows.filter(window => !isWindowHidden(display, account.id, window.id));
    return { kind: 'tone', tone: worstTone(shown.map(window => window.tone)) };
}

export function noticeText(account, notice) {
    if (notice === 'signed_out') return fill(_('Signed out of {provider}'), { provider: account.providerName });
    if (notice === 'no_subscription') return _('No active subscription');
    if (notice === 'error') return account.error?.message ?? null;
    return null;
}

export function sidebarEntries(accounts, display, offline) {
    return accounts.map(account => {
        const title = sidebarTitle(account, accounts);
        const mark = accountMark(account, display, offline);
        return {
            id: account.id,
            provider: account.provider,
            title,
            subtitle: sidebarSubtitle(account, title),
            mark,
            status: mark.kind === 'notice' ? noticeText(account, mark.notice) : null,
            starred: isStarred(display, account.id),
            hidden: account.hidden,
        };
    });
}

export function selectionAfter(ids, selected, previousIds) {
    if (selected !== null && ids.includes(selected)) return selected;
    if (ids.length === 0) return null;
    const index = previousIds.indexOf(selected);
    return ids[Math.min(Math.max(index, 0), ids.length - 1)];
}

export function orderAfterMove(order, id, index) {
    const from = order.indexOf(id);
    if (from < 0 || index < 0 || index >= order.length || index === from) return null;
    return moveItem(order, from, index);
}

export function orderAfterDrop(order, id, slot) {
    const from = order.indexOf(id);
    return orderAfterMove(order, id, from < slot ? slot - 1 : slot);
}

export function canMove(order, id, delta) {
    const index = order.indexOf(id) + delta;
    return order.includes(id) && index >= 0 && index < order.length;
}
