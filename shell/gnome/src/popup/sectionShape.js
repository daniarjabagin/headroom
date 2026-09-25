import { noticeShape } from '../accountStatus.js';
import { headerAccount } from '../combined.js';
import { providerIncident } from '../providerStatus.js';

export function statusShape(ctx, provider) {
    const incident = providerIncident(ctx.providerStatus(), provider);
    return incident ? [incident.tone, ctx.display.density] : null;
}

export function shownWindows(account) {
    return account.windows.filter(window => !window.hidden);
}

export function awaitingFirstData(account) {
    return (
        account.status === 'refreshing' &&
        account.error === null &&
        account.updatedAt === null &&
        shownWindows(account).every(window => window.remainingPercent === null)
    );
}

export function showsSpend(ctx, account) {
    return account.usage !== null && ctx.display.showAccountSpend;
}

export function accountShapeKey(ctx, account, showName) {
    return JSON.stringify({
        kind: 'account',
        showName,
        density: ctx.display.density,
        windows: shownWindows(account).map(window => window.id),
        skeleton: awaitingFirstData(account),
        notice: noticeShape(account, ctx.offline),
        status: statusShape(ctx, account.provider),
        notices: account.notices,
        plan: account.plan,
        label: account.label,
        email: account.email,
        trend: account.usage !== null && ctx.display.showTrend,
        spend: showsSpend(ctx, account),
        balances: account.balances.map(balance => [balance.id, balance.label, balance.kind]),
    });
}

export function combinedShapeKey(ctx, card) {
    return JSON.stringify({
        kind: 'combined',
        density: ctx.display.density,
        status: statusShape(ctx, card.group.provider),
        detail: headerAccount(card).plan,
        members: card.accountIds,
        windows: card.group.windows.map(window => [window.id, window.segments.map(segment => segment.accountId)]),
    });
}

export function cardShapeKey(ctx, card, showName) {
    return card.kind === 'combined' ? combinedShapeKey(ctx, card) : accountShapeKey(ctx, card.account, showName);
}

export function planSections(current, wanted) {
    const free = new Map(current.map(section => [section.id, section]));
    const kept = wanted.map(entry => {
        const section = free.get(entry.id);
        if (!section || section.shapeKey !== entry.shapeKey) return null;
        free.delete(entry.id);
        return section;
    });
    const dropped = current.filter(section => free.get(section.id) === section);
    const inPlace = kept.every((section, index) => section !== null && section === current[index]);
    return { kept, dropped, changed: !inPlace || current.length !== wanted.length };
}
