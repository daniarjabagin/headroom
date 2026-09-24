import * as combined from '../src/combined.js';
import { capacityReading, forecastText, panelCount } from '../src/format.js';
import { paceNote } from '../src/quota.js';
import { setLanguage } from '../src/i18n.js';
import { parseDisplay, displayPatch } from '../src/settings.js';
import { parseState } from '../src/state.js';
import { check } from './check.js';

const NOW = new Date('2026-09-23T10:00:00Z');

function window(id, used, resetsAt, tone, even) {
    return {
        id,
        label: id === 'session' ? 'Session' : 'Weekly',
        used_percent: used,
        remaining_percent: 100 - used,
        resets_at: resetsAt,
        tone,
        pace: { severity: 'healthy', even_pace_percent: even, projected_percent: 60, spare_percent: 40 },
    };
}

function account(id, label, plan, status, windows) {
    return { id, provider: id.split(':')[0], provider_name: 'Codex', label, plan, status, windows };
}

function segment(accountId, label, used, resetsAt, tone) {
    return {
        account_id: accountId,
        label,
        remaining_percent: 100 - used,
        used_percent: used,
        resets_at: resetsAt,
        tone,
    };
}

const GROUP = {
    provider: 'codex',
    provider_name: 'Codex',
    account_ids: ['codex:work', 'codex:personal'],
    accounts: [
        { account_id: 'codex:work', label: 'work', plan: 'Pro' },
        { account_id: 'codex:personal', label: 'personal', plan: 'Plus' },
    ],
    windows: [
        {
            id: 'session',
            label: 'Session',
            capacity_percent: 200,
            remaining_percent: 145,
            used_percent: 55,
            resets_at: '2026-09-23T11:00:00Z',
            tone: 'warning',
            pace: { severity: 'close', even_pace_percent: 40, projected_percent: 180, spare_percent: 20 },
            segments: [
                segment('codex:work', 'work', 5, '2026-09-23T12:41:00Z', 'good'),
                segment('codex:personal', 'personal', 50, '2026-09-23T11:00:00Z', 'warning'),
            ],
        },
    ],
};

function sampleJson(combineAccounts, groups = [GROUP]) {
    return JSON.stringify({
        version: 1,
        display: { combine_accounts: combineAccounts },
        headline: {
            account_id: null,
            window: 'session',
            provider: 'codex',
            provider_name: 'Codex',
            account_label: null,
            window_label: 'Session',
            used_percent: 27.5,
            remaining_percent: 72.5,
            tone: 'warning',
            combined: true,
            account_count: 2,
        },
        accounts: [
            account('claude:main', 'main', 'Max', 'fresh', []),
            account('codex:work', 'work', 'Pro', 'fresh', [window('session', 5, '2026-09-23T12:41:00Z', 'good', 46)]),
            account('codex:old', 'old', 'Plus', 'signed_out', []),
            account('codex:personal', 'personal', 'Plus', 'stale', [
                window('session', 50, '2026-09-23T11:00:00Z', 'warning', 80),
            ]),
        ],
        combined: groups,
    });
}

function kinds(cards) {
    return cards.map(card => [card.kind, card.accountIds]);
}

function testCards() {
    const on = parseState(sampleJson(true));
    check('combined parsed', on.combined[0].windows[0].capacityPercent, 200);
    check('segment parsed', on.combined[0].windows[0].segments[1].accountId, 'codex:personal');
    check('headline combined', [on.headline.combined, on.headline.accountCount], [true, 2]);
    check('cards on', kinds(combined.dashboardCards(on, on.accounts)), [
        ['account', ['claude:main']],
        ['combined', ['codex:work', 'codex:personal']],
        ['account', ['codex:old']],
    ]);
    const off = parseState(sampleJson(false));
    check('cards off', kinds(combined.dashboardCards(off, off.accounts)).length, 4);
    const empty = parseState(sampleJson(true, []));
    check('cards without groups', kinds(combined.dashboardCards(empty, empty.accounts)).length, 4);
    const visible = on.accounts.filter(entry => entry.id !== 'codex:work');
    check('hidden member', kinds(combined.dashboardCards(on, visible))[2], ['combined', ['codex:personal']]);
    const bare = parseState('{"version": 1, "combined": [{"provider": "codex"}]}');
    check('group without accounts dropped', bare.combined, []);
}

function testHeader() {
    setLanguage('en');
    const state = parseState(sampleJson(true));
    const card = combined.dashboardCards(state, state.accounts)[1];
    const header = combined.headerAccount(card);
    check('header detail', header.plan, '2 accounts · Pro · Plus');
    check('header stale member', header.status, 'stale');
    check('header title', header.providerName, 'Codex');
    setLanguage('ru');
    check('ru detail', combined.groupDetail(card.group), '2 аккаунта · Pro · Plus');
    check(
        'ru plurals',
        [1, 2, 5, 21].map(count => combined.accountCountText(count)),
        ['1 аккаунт', '2 аккаунта', '5 аккаунтов', '21 аккаунт']
    );
    setLanguage('en');
    const same = { ...card.group, accounts: card.group.accounts.map(member => ({ ...member, plan: 'Pro' })) };
    check('same plans once', combined.groupDetail(same), '2 accounts · Pro');
}

function testReadings() {
    setLanguage('en');
    check('left of capacity', capacityReading(145, 200, 'left'), '145% left of 200%');
    check('used of capacity', capacityReading(55.4, 200, 'used'), '55% used of 200%');
    check('no reading', capacityReading(null, 200, 'left'), '—');
    setLanguage('ru');
    check('ru left of capacity', capacityReading(145, 200, 'left'), 'Осталось 145% из 200%');
    check('ru used of capacity', capacityReading(55, 200, 'used'), 'Использовано 55% из 200%');
    setLanguage('en');
    check('panel count', panelCount(2), '×2');
    const window = parseState(sampleJson(true)).combined[0].windows[0];
    check('combined left', combined.combinedPercent(window, 'left'), 145);
    check('combined used', combined.combinedPercent(window, 'used'), 55);
    check('combined used fallback', combined.combinedPercent({ ...window, usedPercent: null }, 'used'), 55);
}

function testMeter() {
    const state = parseState(sampleJson(true));
    const card = combined.dashboardCards(state, state.accounts)[1];
    const window = card.group.windows[0];
    const display = parseDisplay({ combine_accounts: true });
    check('segments left', combined.combinedMeterState(window, card.members, display), {
        segments: [
            { fraction: 0.95, tone: 'ok', tick: 0.54 },
            { fraction: 0.5, tone: 'warn', tick: 0.19999999999999996 },
        ],
    });
    const used = { ...display, valueMode: 'used', showForecast: false };
    check('segments used', combined.combinedMeterState(window, card.members, used), {
        segments: [
            { fraction: 0.05, tone: 'ok', tick: null },
            { fraction: 0.5, tone: 'warn', tick: 0.8 },
        ],
    });
    const orphan = {
        ...window,
        segments: [{ ...window.segments[0], accountId: 'codex:gone', remainingPercent: null }],
    };
    check('segment without data', combined.combinedMeterState(orphan, card.members, display), {
        segments: [{ fraction: 0, tone: 'none', tick: null }],
    });
}

function testLayout() {
    check('two segments', combined.segmentLayout(2, 264, 2), [
        { x: 0, width: 131 },
        { x: 133, width: 131 },
    ]);
    check('uneven', combined.segmentLayout(3, 100, 2), [
        { x: 0, width: 32 },
        { x: 34, width: 32 },
        { x: 68, width: 32 },
    ]);
    check('remainder', combined.segmentLayout(3, 101, 2), [
        { x: 0, width: 33 },
        { x: 35, width: 32 },
        { x: 69, width: 32 },
    ]);
    check('none', combined.segmentLayout(0, 100, 2), []);
    check('too narrow', combined.segmentLayout(2, 1, 2), [
        { x: 0, width: 0 },
        { x: 2, width: 0 },
    ]);
}

function testBreakdown() {
    setLanguage('en');
    const state = parseState(sampleJson(true));
    const card = combined.dashboardCards(state, state.accounts)[1];
    const display = parseDisplay(null);
    check('breakdown', combined.breakdownEntries(card.group.windows[0], card.members, NOW, display), [
        { name: 'work', reading: '95% left', reset: 'Resets in 2h 41m' },
        { name: 'personal', reading: '50% left', reset: 'Resets in 1h 0m' },
    ]);
    const used = { ...display, valueMode: 'used' };
    const unnamed = { ...card.group.windows[0].segments[0], label: null, remainingPercent: null };
    check(
        'breakdown fallback',
        combined.breakdownEntries({ ...card.group.windows[0], segments: [unnamed] }, card.members, NOW, used),
        [{ name: 'work', reading: '—', reset: 'No data' }]
    );
}

function testForecast() {
    setLanguage('en');
    const window = parseState(sampleJson(true)).combined[0].windows[0];
    const display = parseDisplay(null);
    check('capacity forecast left', forecastText(window, NOW, display), 'At this pace: ~20% of 200% left at reset');
    check(
        'capacity forecast used',
        forecastText(window, NOW, { ...display, valueMode: 'used' }),
        'At this pace: ~180% of 200% used at reset'
    );
    const over = { ...window, pace: { ...window.pace, severity: 'running_out', sparePercent: null, runsOutAt: null } };
    check('combined runs out', forecastText(over, NOW, display), 'At this pace: runs out before reset');
    check('combined over pace note', paceNote(over, NOW, false), { flame: true, text: 'Over pace' });
    setLanguage('ru');
    check(
        'ru capacity forecast',
        forecastText(window, NOW, display),
        'При текущем темпе к сбросу останется ~20% из 200%'
    );
    setLanguage('en');
}

function testSetting() {
    check('combine default', parseDisplay(null).combineAccounts, false);
    check('combine on', parseDisplay({ combine_accounts: true }).combineAccounts, true);
    check('combine patch', displayPatch({ combineAccounts: true }), { display: { combine_accounts: true } });
}

export function testCombinedSnapshot(json) {
    setLanguage('en');
    const state = parseState(json);
    const visible = state.accounts.filter(entry => !entry.hidden);
    const cards = combined.dashboardCards(state, visible);
    check('snapshot cards', kinds(cards), [
        ['combined', ['codex:work', 'codex:personal']],
        ['account', ['claude:main']],
    ]);
    const [session, weekly] = cards[0].group.windows;
    check(
        'snapshot pooled reading',
        capacityReading(session.remainingPercent, session.capacityPercent, 'left'),
        '125% left of 200%'
    );
    check(
        'snapshot pooled forecast',
        forecastText(session, NOW, state.display),
        'At this pace: ~68% of 200% left at reset'
    );
    check('snapshot single segment', [weekly.capacityPercent, weekly.segments.length], [100, 1]);
    check('snapshot single forecast', forecastText(weekly, NOW, state.display), 'At this pace: ~16% left at reset');
    check('snapshot header', combined.headerAccount(cards[0]).plan, '2 accounts · Pro · Plus');
}

export function testCombined() {
    testCards();
    testHeader();
    testReadings();
    testMeter();
    testLayout();
    testBreakdown();
    testForecast();
    testSetting();
}
