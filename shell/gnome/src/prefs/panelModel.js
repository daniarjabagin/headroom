import { percentLeft, windowLabel } from '../format.js';
import { _, fill } from '../i18n.js';
import { MAX_PANEL_LIMITS } from '../settings.js';

const KEY_SEPARATOR = '\n';

export function limitKey(accountId, windowId) {
    return `${accountId}${KEY_SEPARATOR}${windowId}`;
}

export function panelRowsShown(mode, supported) {
    if (!supported) return { shows: false, limit: true, limits: false, indicator: false, label: true, position: false };
    const icon = mode === 'icon';
    return {
        shows: true,
        limit: mode === 'headline',
        limits: mode === 'several',
        indicator: !icon,
        label: !icon,
        position: true,
    };
}

function choiceFor(account, window) {
    const who = account.label ?? account.email;
    const reading = window.remainingPercent === null ? null : percentLeft(window.remainingPercent);
    return {
        key: limitKey(account.id, window.id),
        accountId: account.id,
        window: window.id,
        provider: account.provider,
        title: `${account.providerName} — ${windowLabel(window.id, window.label)}`,
        subtitle: [who, reading].filter(Boolean).join(' · '),
    };
}

export function limitChoices(accounts, chosen) {
    const all = accounts
        .filter(account => !account.hidden)
        .flatMap(account => account.windows.filter(window => !window.hidden).map(window => choiceFor(account, window)));
    const chosenKeys = chosen.map(limit => limitKey(limit.accountId, limit.window));
    const picked = chosenKeys.map(key => all.find(choice => choice.key === key)).filter(Boolean);
    const rest = all.filter(choice => !chosenKeys.includes(choice.key));
    return [
        ...picked.map(choice => ({ ...choice, checked: true })),
        ...rest.map(choice => ({ ...choice, checked: false })),
    ];
}

export function limitsAfter(chosen, limit, checked) {
    const key = limitKey(limit.accountId, limit.window);
    const others = chosen.filter(entry => limitKey(entry.accountId, entry.window) !== key);
    if (!checked || others.length >= MAX_PANEL_LIMITS) return others;
    return [...others, { accountId: limit.accountId, window: limit.window }];
}

export function limitsSubtitle(count) {
    if (count === 0) return _('None chosen · the two most critical show');
    return fill(_('{count} of {max} chosen · shown in this order'), { count, max: MAX_PANEL_LIMITS });
}

export function canChoose(chosenCount, checked) {
    return checked || chosenCount < MAX_PANEL_LIMITS;
}
