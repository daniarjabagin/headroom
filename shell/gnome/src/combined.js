import { percentReading, readingPercent } from './format.js';
import { fill, n_ } from './i18n.js';
import { accountName } from './providers.js';
import { fillFraction, hasData, meterTone, tickPosition, trailingText } from './quota.js';

const DETAIL_SEPARATOR = ' · ';

function accountCard(account) {
    return { kind: 'account', id: account.id, accountIds: [account.id], account };
}

function combinedCard(group, accounts) {
    const members = accounts.filter(account => group.accountIds.includes(account.id));
    return {
        kind: 'combined',
        id: `combined:${group.provider}:${group.accountIds.join(',')}`,
        accountIds: members.map(account => account.id),
        group,
        members,
    };
}

export function dashboardCards(state, accounts) {
    const groups = state.display.combineAccounts ? state.combined : [];
    const groupOf = new Map(groups.flatMap(group => group.accountIds.map(id => [id, group])));
    const placed = new Set();
    const cards = [];
    for (const account of accounts) {
        const group = groupOf.get(account.id);
        if (!group) cards.push(accountCard(account));
        else if (!placed.has(group)) {
            placed.add(group);
            cards.push(combinedCard(group, accounts));
        }
    }
    return cards;
}

export function accountCountText(count) {
    return fill(n_('{count} account', '{count} accounts', count), { count });
}

export function groupDetail(group) {
    const plans = [...new Set(group.accounts.map(member => member.plan).filter(plan => plan !== null))];
    return [accountCountText(group.accountIds.length), ...plans].join(DETAIL_SEPARATOR);
}

function groupStatus(members) {
    if (members.some(account => account.status === 'refreshing')) return 'refreshing';
    if (members.some(account => account.status === 'stale' || account.status === 'error')) return 'stale';
    return 'fresh';
}

function oldestUpdate(members) {
    const stamps = members.map(account => account.updatedAt).filter(stamp => stamp !== null);
    return stamps.length === 0 ? null : new Date(Math.min(...stamps));
}

export function headerAccount(card) {
    const { group, members } = card;
    return {
        id: card.id,
        provider: group.provider,
        providerName: group.providerName,
        label: null,
        email: null,
        plan: groupDetail(group),
        status: groupStatus(members),
        error: null,
        updatedAt: oldestUpdate(members),
    };
}

export function combinedPercent(window, valueMode) {
    if (window.remainingPercent === null) return null;
    if (valueMode === 'used') return window.usedPercent ?? window.capacityPercent - window.remainingPercent;
    return window.remainingPercent;
}

function memberWindow(members, accountId, windowId) {
    const member = members.find(account => account.id === accountId);
    return member?.windows.find(window => window.id === windowId) ?? null;
}

export function segmentTick(members, segment, windowId, display) {
    const window = memberWindow(members, segment.accountId, windowId);
    return window ? tickPosition(window, display) : null;
}

export function combinedMeterState(window, members, display) {
    return {
        segments: window.segments.map(segment => ({
            fraction: fillFraction(segment, display.valueMode),
            tone: meterTone(segment),
            tick: segmentTick(members, segment, window.id, display),
        })),
    };
}

export function segmentLayout(count, width, gap) {
    if (count <= 0) return [];
    const usable = Math.max(0, width - gap * (count - 1));
    const base = Math.floor(usable / count);
    const extra = usable - base * count;
    const layout = [];
    let x = 0;
    for (let index = 0; index < count; index++) {
        const segmentWidth = base + (index < extra ? 1 : 0);
        layout.push({ x, width: segmentWidth });
        x += segmentWidth + gap;
    }
    return layout;
}

function segmentName(segment, members) {
    const member = members.find(account => account.id === segment.accountId);
    return segment.label ?? (member ? accountName(member) : segment.accountId);
}

export function breakdownEntries(window, members, now, display) {
    return window.segments.map(segment => ({
        name: segmentName(segment, members),
        reading: percentReading(
            hasData(segment) ? readingPercent(segment, display.valueMode) : null,
            display.valueMode
        ),
        reset: trailingText(segment, now, display.resetFormat),
    }));
}
