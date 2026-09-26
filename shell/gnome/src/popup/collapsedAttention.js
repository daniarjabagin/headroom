import { accountNotice } from '../accountStatus.js';
import { fill, n_ } from '../i18n.js';

const TONE_RANK = { neutral: 0, good: 0, warning: 1, critical: 2 };
const CALM = { kind: 'none', tone: null, count: 0 };

function cardAccounts(card) {
    return card.kind === 'combined' ? card.members : [card.account];
}

function cardWindows(card) {
    const windows = card.kind === 'combined' ? card.group.windows : card.account.windows;
    return windows.filter(window => !window.hidden);
}

function rank(tone) {
    return TONE_RANK[tone] ?? 0;
}

function worstTone(tones) {
    return tones.reduce((worst, tone) => (rank(tone) > rank(worst) ? tone : worst), 'neutral');
}

function isAlarming(tone) {
    return rank(tone) > 0;
}

function cardAttention(card, offline) {
    return {
        notices: cardAccounts(card)
            .map(account => accountNotice(account, offline))
            .filter(notice => notice !== null),
        tone: worstTone(cardWindows(card).map(window => window.tone)),
    };
}

export function collapsedAttention(cards, offline) {
    const entries = cards.map(card => cardAttention(card, offline));
    const count = entries.filter(entry => entry.notices.length > 0 || isAlarming(entry.tone)).length;
    const notices = entries.flatMap(entry => entry.notices);
    if (notices.length > 0) return { kind: 'notice', tone: notices.includes('error') ? 'critical' : 'warning', count };
    const tone = worstTone(entries.map(entry => entry.tone));
    return isAlarming(tone) ? { kind: 'tone', tone, count } : CALM;
}

export function attentionText(attention) {
    if (attention.kind === 'none') return null;
    const { count } = attention;
    return fill(n_('{count} needs attention', '{count} need attention', count), { count });
}
