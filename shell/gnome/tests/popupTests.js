import { barParts, modelsTable, preciseShareText, projectsTable, wholeShareText } from '../src/breakdown.js';
import { setLanguage } from '../src/i18n.js';
import { cardMenuItems, hostOf, SEPARATOR, shownLinks } from '../src/popup/cardMenu.js';
import { compactTrailing, paceTip, resetHint, trailingLabel, valueHint } from '../src/popup/limitTexts.js';
import { footerLines, footerNeedsSeconds } from '../src/popup/refreshTexts.js';
import { shareDate, shareModel, shareText } from '../src/popup/share/shareModel.js';
import * as view from '../src/popup/spendView.js';
import { statusAge, statusDetail, statusStarted, statusTitle, statusTone } from '../src/popup/statusTexts.js';
import { parseDisplay } from '../src/settings.js';
import { check } from './check.js';

const NOW = new Date('2026-09-25T12:00:00Z');
const MINUTE = 60 * 1000;
const HOUR = 60 * MINUTE;

function provider(id, costMicros, totalTokens, models, modelsOther = null) {
    return {
        provider: id,
        providerName: id,
        costMicros,
        totalTokens,
        costPerMtokMicros: 2_000_000,
        models,
        modelsOther,
    };
}

function model(name, costMicros, totalTokens) {
    return { model: name, costMicros, totalTokens, partial: false };
}

const PERIOD = {
    costMicros: 100_000_000,
    totalTokens: 50_000_000,
    costPerMtokMicros: 2_000_000,
    partial: false,
    providers: [
        provider('claude', 60_000_000, 20_000_000, [
            model('opus', 40_000_000, 8_000_000),
            model('sonnet', 20_000_000, 12_000_000),
        ]),
        provider(
            'codex',
            40_000_000,
            30_000_000,
            [
                model('gpt-5', 30_000_000, 20_000_000),
                model('mini', 5_000_000, 5_000_000),
                model('nano', 3_000_000, 3_000_000),
                model('o4', 1_000_000, 1_000_000),
            ],
            { count: 2, costMicros: 1_000_000, totalTokens: 1_000_000, partial: false }
        ),
    ],
    projects: [
        {
            project: '~/code/app',
            costMicros: 70_000_000,
            totalTokens: 30_000_000,
            sharePermille: 700,
            providers: [
                { provider: 'claude', providerName: 'Claude', costMicros: 50_000_000, totalTokens: 10_000_000 },
                { provider: 'codex', providerName: 'Codex', costMicros: 20_000_000, totalTokens: 20_000_000 },
            ],
        },
    ],
    projectsOther: { count: 4, costMicros: 30_000_000, totalTokens: 20_000_000, sharePermille: 300, partial: false },
};

function testSpendView() {
    const spend = { today: PERIOD, yesterday: PERIOD, last7Days: null, last30Days: PERIOD };
    check('periods without 7 days', view.periodChoices(spend), ['today', 'yesterday', '30d']);
    check('periods with 7 days', view.periodChoices({ ...spend, last7Days: PERIOD }), [
        'today',
        'yesterday',
        '7d',
        '30d',
    ]);
    check('missing period falls back', view.shownPeriod(spend, '7d'), '30d');
    check('period titles', ['today', '7d', '30d'].map(view.periodTitle), ['Today', '7 Days', '30 Days']);
    check('unit titles', view.UNITS.map(view.unitTitle), ['Total Spend', 'Total Tokens', 'Cost per MTok']);
    check('cost center', view.centerText(view.centerValue(PERIOD, 'cost'), 'cost'), '$100');
    check(
        'tokens center',
        [view.centerText(view.centerValue(PERIOD, 'tokens'), 'tokens'), view.centerCaption('tokens')],
        ['50M', 'tokens']
    );
    check(
        'rate center',
        [view.centerText(2_000_000, 'cost_per_mtok'), view.centerCaption('cost_per_mtok')],
        ['$2.00', 'blended']
    );
    check('rate center unpriced', view.centerText(null, 'cost_per_mtok'), '—');
}

function testSpendUnitsView() {
    check(
        'token slices',
        view.unitSlices(PERIOD, 'tokens').map(slice => slice.value),
        [20_000_000, 30_000_000]
    );
    check(
        'rate slices follow tokens',
        view.unitSlices(PERIOD, 'cost_per_mtok').map(slice => slice.value),
        [20_000_000, 30_000_000]
    );
    check(
        'legend rate',
        view.legendText(view.legendValue(PERIOD.providers[0], 'cost_per_mtok'), 'cost_per_mtok'),
        '$2.00'
    );
    check('legend tokens', view.legendText(view.legendValue(PERIOD.providers[1], 'tokens'), 'tokens'), '30M');
    check('body key has unit', view.spendBodyKey(PERIOD, 'cost') !== view.spendBodyKey(PERIOD, 'tokens'), true);
}

function testBreakdownTables() {
    const models = modelsTable(PERIOD, 'cost');
    check('old daemon lists every provider model', models.rows.map(row => row.name).slice(0, 6), [
        'opus',
        'gpt-5',
        'sonnet',
        'mini',
        'nano',
        'o4',
    ]);
    const other = models.rows[6];
    check(
        'old daemon other sums provider others without a rate',
        [other.other, other.costMicros, other.totalTokens, other.costPerMtokMicros],
        [2, 1_000_000, 1_000_000, null]
    );
    check('model count', models.count, 8);
    check(
        'model shares',
        models.rows.map(row => row.sharePermille),
        [400, 300, 200, 50, 30, 10, 10]
    );
    const projects = projectsTable(PERIOD, 'cost');
    check(
        'project rows',
        projects.rows.map(row => [row.name, row.other, row.sharePermille]),
        [
            ['~/code/app', null, 700],
            ['Other', 4, 300],
        ]
    );
    check('project count', projects.count, 5);
    check('project bar split by provider', barParts(projects.rows[0], PERIOD, 'cost'), [
        { series: 'claude', permille: 500 },
        { series: 'codex', permille: 200 },
    ]);
    check('other project bar', barParts(projects.rows[1], PERIOD, 'cost'), [{ series: null, permille: 300 }]);
    check('no projects', projectsTable({ ...PERIOD, projects: null }, 'cost'), null);
    check('share texts', [wholeShareText(623), preciseShareText(323), preciseShareText(40)], ['62%', '32.3%', '4.0%']);
}

function window(overrides = {}) {
    return {
        id: 'session',
        label: 'Session',
        usedPercent: 24,
        remainingPercent: 76,
        resetsAt: new Date(NOW.getTime() + 3 * HOUR + 11 * MINUTE),
        hidden: false,
        tone: 'good',
        pace: { severity: 'healthy', evenPacePercent: 36, projectedPercent: 67, sparePercent: 33, runsOutAt: null },
        ...overrides,
    };
}

function testLimitTexts() {
    const session = window();
    check('trailing', trailingLabel(session, NOW, 'countdown', false), 'Resets in 3h 11m');
    check('compact countdown', compactTrailing(session, NOW, 'countdown', false), '3h 11m');
    const soon = window({ resetsAt: new Date(NOW.getTime() + 4 * MINUTE + 5000) });
    check('compact live seconds', compactTrailing(soon, NOW, 'countdown', false), '4m 05s');
    const at = new Date(2026, 8, 25, 14, 30);
    const local = new Date(2026, 8, 25, 12, 0);
    check('compact exact 24h', compactTrailing(window({ resetsAt: at }), local, 'exact', false), 'today at 14:30');
    check('compact exact 12h', compactTrailing(window({ resetsAt: at }), local, 'exact', true), 'today at 2:30 PM');
    check(
        'trailing exact 12h',
        trailingLabel(window({ resetsAt: at }), local, 'exact', true),
        'Resets today at 2:30 PM'
    );
    check('no data', compactTrailing(window({ remainingPercent: null }), NOW, 'countdown', false), 'No data');
    check(
        'hints',
        [valueHint('left'), valueHint('used'), resetHint('countdown'), resetHint('exact')],
        [
            'Click to show used',
            'Click to show what is left',
            'Click to show the reset time',
            'Click to show the countdown',
        ]
    );
    const display = parseDisplay(null);
    check('pace tip', paceTip(session, NOW, display, false), 'At this pace: ~33% left at reset');
}

function status(overrides = {}) {
    return {
        provider: 'claude',
        indicator: 'minor',
        tone: 'warning',
        title: 'Elevated errors on Claude API',
        stage: 'investigating',
        startedAt: new Date(NOW.getTime() - 38 * MINUTE),
        url: 'https://status.claude.com',
        ...overrides,
    };
}

function testStatusTexts() {
    check('incident title', statusTitle(status()), 'Incident · Elevated errors on Claude API');
    check(
        'outage title',
        statusTitle(status({ indicator: 'critical', tone: 'critical', title: null })),
        'Major outage'
    );
    check('tones', [statusTone(status()), statusTone(status({ tone: 'critical' }))], ['warning', 'error']);
    check('detail', statusDetail(status()), 'Investigating — requests may fail or be slow.');
    check('no stage', statusDetail(status({ stage: null })), null);
    check('started', statusStarted(status(), NOW), 'Started 38m ago');
    check('age', statusAge(status(), NOW), '· 38m');
    check('unknown start', statusStarted(status({ startedAt: null }), NOW), null);
}

function state(overrides = {}) {
    return {
        offline: false,
        lastSuccessAt: new Date(2026, 8, 25, 12, 25),
        nextRefreshAt: new Date(2026, 8, 25, 12, 30),
        accounts: [],
        ...overrides,
    };
}

function testFooterLines() {
    const now = new Date(2026, 8, 25, 12, 26);
    const idle = footerLines({ kind: 'ready', state: state() }, now, false);
    check('idle', [idle.first, idle.second.text], [null, 'Updated 12:25 · next at 12:30']);
    check(
        'idle 12h',
        footerLines({ kind: 'ready', state: state() }, now, true).second.text,
        'Updated 12:25 PM · next at 12:30 PM'
    );
    const liveAccount = {
        hidden: false,
        status: 'fresh',
        providerName: 'Claude',
        refresh: { mode: 'live', intervalSecs: 60 },
    };
    const live = footerLines({ kind: 'ready', state: state({ accounts: [liveAccount] }) }, now, false);
    check(
        'live',
        [live.first.text, live.second.text, live.second.kind],
        ['Updated 1m ago', 'Live — every minute while Claude is active', 'live']
    );
    check('live tip', live.tip.startsWith('Claude is writing new usage logs'), true);
    const offline = footerLines(
        { kind: 'ready', state: state({ offline: true, nextRefreshAt: new Date(now.getTime() + 45_000) }) },
        now,
        false
    );
    check(
        'offline',
        [offline.first.text, offline.first.kind, offline.second.text],
        ['Outdated · updated 1m ago', 'notice', 'Offline — retrying in 45s']
    );
    check(
        'unavailable',
        footerLines({ kind: 'unavailable', state: null }, now, false).second.text,
        'Service not running'
    );
}

function testFooterSeconds() {
    const now = new Date(2026, 8, 25, 12, 26);
    const retryIn = ms => ({
        kind: 'ready',
        state: state({ offline: true, nextRefreshAt: new Date(now.getTime() + ms) }),
    });
    check(
        'footer second ticks',
        [
            footerNeedsSeconds(retryIn(45_000), now),
            footerNeedsSeconds(retryIn(2 * 60 * 60 * 1000), now),
            footerNeedsSeconds(retryIn(-1000), now),
            footerNeedsSeconds({ kind: 'ready', state: state() }, now),
            footerNeedsSeconds({ kind: 'loading', state: null }, now),
        ],
        [true, false, false, false, false]
    );
}

function accountCard() {
    const account = {
        id: 'codex:1',
        provider: 'codex',
        providerName: 'Codex',
        plan: 'Pro',
        label: 'work',
        email: 'dev@example.com',
        windows: [
            window(),
            window({ id: 'weekly', label: 'Weekly', remainingPercent: 32, usedPercent: 68, tone: 'warning' }),
        ],
    };
    return { kind: 'account', id: account.id, accountIds: [account.id], account };
}

function testShareModel() {
    const ctx = { display: parseDisplay(null), now: NOW, hour12: false };
    const shared = shareModel(accountCard(), ctx);
    check('identity has no email', [shared.providerName, shared.detail], ['Codex', 'Pro']);
    check('weekly headline', [shared.headline.percent, shared.headline.word], ['32%', 'left']);
    check('headline sub', shared.headline.sub.startsWith('weekly · resets in'), true);
    check(
        'rows',
        shared.rows.map(row => [row.label, row.reading]),
        [
            ['Session', '76% left'],
            ['Weekly', '32% left'],
        ]
    );
    check('date', shareDate('Codex', new Date(2026, 8, 25, 10)), 'CODEX LIMITS · 25 SEP 2026');
    const text = shareText(shared);
    check('text', text.split('\n').slice(0, 2), ['Codex · Pro', 'Session: 76% left · Resets in 3h 11m']);
    check('text has no email', text.includes('example.com') || text.includes('work'), false);
    const empty = { ...accountCard(), account: { ...accountCard().account, windows: [] } };
    check('nothing to share', shareModel(empty, ctx), null);
    setLanguage('ru');
    try {
        check('ru date', shareDate('Codex', new Date(2026, 8, 25, 10)), 'ЛИМИТЫ CODEX · 25 СЕНТ. 2026');
    } finally {
        setLanguage('en');
    }
}

const LINKS = {
    status: 'https://status.openai.com',
    dashboard: 'https://chatgpt.com/codex',
    usage: 'https://chatgpt.com/codex',
};

function testCardMenu() {
    const links = LINKS;
    check('host', hostOf('https://www.githubstatus.com/incidents/1'), 'githubstatus.com');
    check(
        'usage equal to dashboard is skipped',
        shownLinks(links).map(link => link.key),
        ['status', 'dashboard']
    );
}

function testCardMenuItems() {
    const links = LINKS;
    const items = cardMenuItems({
        providerName: 'Codex',
        canHide: true,
        canStar: true,
        starred: false,
        links,
        canShare: true,
    });
    check(
        'menu',
        items.map(item => (item === SEPARATOR ? '-' : item.id)),
        ['refresh', 'hide', 'star', '-', 'link:status', 'link:dashboard', '-', 'share', 'copy']
    );
    check('accel', items[4].accel, 'status.openai.com');
    const bare = cardMenuItems({
        providerName: 'X',
        canHide: false,
        canStar: false,
        starred: false,
        links: null,
        canShare: false,
    });
    check(
        'bare menu',
        bare.map(item => item.id),
        ['refresh']
    );
    const starred = cardMenuItems({
        providerName: 'X',
        canHide: false,
        canStar: true,
        starred: true,
        links: null,
        canShare: false,
    });
    check('unstar', starred[1].label, 'Show on demand');
}

export function testPopup() {
    testSpendView();
    testSpendUnitsView();
    testBreakdownTables();
    testLimitTexts();
    testStatusTexts();
    testFooterLines();
    testFooterSeconds();
    testShareModel();
    testCardMenu();
    testCardMenuItems();
}
