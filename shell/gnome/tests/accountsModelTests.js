import { setLanguage } from '../src/i18n.js';
import { displayPatch, parseDisplay } from '../src/settings.js';
import {
    accountMark,
    canMove,
    detailSubtitle,
    orderAfterDrop,
    orderAfterMove,
    selectionAfter,
    sidebarEntries,
    sidebarTitle,
} from '../src/prefs/accountsModel.js';
import { hasBreakdownSetting } from '../src/prefs/breakdownSetting.js';
import { check } from './check.js';

function window(id, tone) {
    return { id, label: null, remainingPercent: 50, tone };
}

function account(id, provider, extra = {}) {
    return {
        id,
        provider,
        providerName: provider === 'claude' ? 'Claude' : 'Codex',
        label: null,
        email: `${id.split(':')[1]}@example.com`,
        plan: 'Pro',
        owner: 'cli',
        status: 'fresh',
        error: null,
        recovery: null,
        hidden: false,
        windows: [window('session', 'good'), window('weekly', 'warning')],
        ...extra,
    };
}

const WORK = account('codex:work', 'codex', { label: 'work' });
const HOME = account('codex:home', 'codex');
const CLAUDE = account('claude:me', 'claude', { owner: 'headroom' });
const ACCOUNTS = [WORK, HOME, CLAUDE];

function testTitles() {
    check(
        'titles name the account only among siblings or with a label',
        ACCOUNTS.map(entry => sidebarTitle(entry, ACCOUNTS)),
        ['Codex · work', 'Codex · home@example.com', 'Claude']
    );
    const entries = sidebarEntries(ACCOUNTS, parseDisplay(null), false);
    check(
        'subtitles skip an email already in the title',
        entries.map(entry => entry.subtitle),
        ['Pro · work@example.com', 'Pro', 'Pro · me@example.com']
    );
    check('detail subtitle', detailSubtitle(CLAUDE), 'Claude · Pro · added in Headroom');
    check('detail subtitle with label', detailSubtitle(WORK), 'Codex · Pro · work@example.com');
}

function testMarks() {
    const display = parseDisplay({ hidden_windows: { 'codex:home': ['weekly'] }, starred_accounts: ['claude:me'] });
    check('worst shown tone', accountMark(WORK, display, false), { kind: 'tone', tone: 'warning' });
    check('hidden windows do not tint', accountMark(HOME, display, false), { kind: 'tone', tone: 'good' });
    check('no windows is neutral', accountMark({ ...WORK, windows: [] }, display, false).tone, 'neutral');
    const signedOut = { ...CLAUDE, status: 'signed_out' };
    check('signed out notice', accountMark(signedOut, display, false), { kind: 'notice', notice: 'signed_out' });
    const offline = { ...WORK, status: 'error', error: { kind: 'network', message: 'offline' } };
    check('network errors stay quiet offline', accountMark(offline, display, true).kind, 'tone');
    check('network errors show online', accountMark(offline, display, false).notice, 'error');
    const entries = sidebarEntries([WORK, signedOut], display, false);
    check('status text', [entries[0].status, entries[1].status], [null, 'Signed out of Claude']);
    check('starred', [entries[0].starred, entries[1].starred], [false, true]);
}

function testSelection() {
    check('keeps the selection', selectionAfter(['a', 'b'], 'b', ['a', 'b']), 'b');
    check('first when nothing selected', selectionAfter(['a', 'b'], null, []), 'a');
    check('neighbour after removal', selectionAfter(['a', 'c'], 'b', ['a', 'b', 'c']), 'c');
    check('last after removing the last', selectionAfter(['a'], 'b', ['a', 'b']), 'a');
    check('nothing when empty', selectionAfter([], 'a', ['a']), null);
}

function testOrdering() {
    const order = ['a', 'b', 'c'];
    check('move down', orderAfterMove(order, 'a', 1), ['b', 'a', 'c']);
    check('move out of range', [orderAfterMove(order, 'a', -1), orderAfterMove(order, 'c', 3)], [null, null]);
    check('unknown id', orderAfterMove(order, 'x', 0), null);
    check('drop below the next row', orderAfterDrop(order, 'a', 2), ['b', 'a', 'c']);
    check('drop at the top', orderAfterDrop(order, 'c', 0), ['c', 'a', 'b']);
    check('drop on itself', orderAfterDrop(order, 'b', 1), null);
    check(
        'move bounds',
        [canMove(order, 'a', -1), canMove(order, 'a', 1), canMove(order, 'c', 1)],
        [false, true, false]
    );
}

function testBreakdownSetting() {
    check('old daemon lacks the key', hasBreakdownSetting({ display: {} }), false);
    check('no settings', hasBreakdownSetting(null), false);
    check('stored off', hasBreakdownSetting({ display: { show_breakdown: false } }), true);
    check('patch', displayPatch({ showBreakdown: false }), { display: { show_breakdown: false } });
}

export function testAccountsModel() {
    setLanguage('en');
    testTitles();
    testMarks();
    testSelection();
    testOrdering();
    testBreakdownSetting();
}
