import { barParts, breakdownValue, modelsTable, preciseShareText, projectsTable } from '../src/breakdown.js';
import { setLanguage } from '../src/i18n.js';
import { RU } from '../src/locale/ru.js';
import { cardMenuItems } from '../src/popup/cardMenu.js';
import { needsToolsHint, isEmptyState } from '../src/popup/dashboardPlan.js';
import { footerLines } from '../src/popup/refreshTexts.js';
import { legendTitle } from '../src/popup/spendView.js';
import { check } from './check.js';

function model(name, costMicros, totalTokens, costPerMtokMicros) {
    return { model: name, costMicros, totalTokens, partial: false, costPerMtokMicros };
}

const PERIOD = {
    costMicros: 60_000_000,
    totalTokens: 40_000_000,
    costPerMtokMicros: 1_500_000,
    partial: false,
    providers: [
        {
            provider: 'claude',
            providerName: 'Claude',
            costMicros: 50_000_000,
            totalTokens: 10_000_000,
            models: [model('opus', 45_000_000, 4_000_000, 11_250_000), model('haiku', 5_000_000, 6_000_000, 833_333)],
            modelsOther: null,
        },
        {
            provider: 'codex',
            providerName: 'Codex',
            costMicros: 10_000_000,
            totalTokens: 30_000_000,
            models: [model('gpt-5', 10_000_000, 30_000_000, 333_333)],
            modelsOther: null,
        },
    ],
    projects: [
        {
            project: '~/big-cost',
            costMicros: 40_000_000,
            totalTokens: 5_000_000,
            partial: false,
            sharePermille: 666,
            costPerMtokMicros: 8_000_000,
            providers: [{ provider: 'claude', providerName: 'Claude', costMicros: 40_000_000, totalTokens: 5_000_000 }],
        },
        {
            project: '~/big-tokens',
            costMicros: 20_000_000,
            totalTokens: 35_000_000,
            partial: false,
            sharePermille: 333,
            costPerMtokMicros: null,
            providers: [{ provider: 'codex', providerName: 'Codex', costMicros: 20_000_000, totalTokens: 35_000_000 }],
        },
    ],
    projectsOther: null,
};

function testModelUnits() {
    const cost = modelsTable(PERIOD, 'cost');
    check(
        'cost order',
        cost.rows.map(row => row.name),
        ['opus', 'gpt-5', 'haiku']
    );
    const tokens = modelsTable(PERIOD, 'tokens');
    check(
        'tokens order',
        tokens.rows.map(row => row.name),
        ['gpt-5', 'haiku', 'opus']
    );
    check(
        'tokens shares',
        tokens.rows.map(row => row.sharePermille),
        [750, 150, 100]
    );
    check(
        'tokens values',
        tokens.rows.map(row => breakdownValue(row, 'tokens')),
        ['30M', '6M', '4M']
    );
    const rate = modelsTable(PERIOD, 'cost_per_mtok');
    check(
        'rate follows token order',
        rate.rows.map(row => row.name),
        ['gpt-5', 'haiku', 'opus']
    );
    check(
        'rate values',
        rate.rows.map(row => breakdownValue(row, 'cost_per_mtok')),
        ['$0.33', '$0.83', '$11.25']
    );
    check('rate bar by tokens', barParts(rate.rows[0], PERIOD, 'cost_per_mtok'), [{ series: 'codex', permille: 750 }]);
}

function testModelTies() {
    const tied = {
        ...PERIOD,
        providers: [{ ...PERIOD.providers[0], models: [model('b', 2, 5, null), model('a', 1, 5, null)] }],
    };
    check(
        'token ties by name',
        modelsTable(tied, 'tokens').rows.map(row => row.name),
        ['a', 'b']
    );
    check('missing rate is a dash', breakdownValue(modelsTable(tied, 'cost_per_mtok').rows[0], 'cost_per_mtok'), '—');
}

function testProjectUnits() {
    const cost = projectsTable(PERIOD, 'cost');
    check(
        'project cost order',
        cost.rows.map(row => [row.name, row.sharePermille]),
        [
            ['~/big-cost', 666],
            ['~/big-tokens', 333],
        ]
    );
    const tokens = projectsTable(PERIOD, 'tokens');
    check(
        'project token order and share',
        tokens.rows.map(row => [row.name, row.sharePermille]),
        [
            ['~/big-tokens', 875],
            ['~/big-cost', 125],
        ]
    );
    const rate = projectsTable(PERIOD, 'cost_per_mtok');
    check(
        'project rate from payload',
        rate.rows.map(row => breakdownValue(row, 'cost_per_mtok')),
        ['—', '$8.00']
    );
    const folded = { count: 3, costMicros: 1, totalTokens: 50_000_000, sharePermille: 1, costPerMtokMicros: 366_667 };
    const other = projectsTable({ ...PERIOD, projectsOther: folded }, 'tokens');
    check('other stays last', other.rows.map(row => row.name).at(-1), 'Other');
    check('other rate from payload', breakdownValue(other.rows.at(-1), 'cost_per_mtok'), '$0.37');
    const unrated = projectsTable({ ...PERIOD, projectsOther: { ...folded, costPerMtokMicros: null } }, 'cost');
    check('other without rate', breakdownValue(unrated.rows.at(-1), 'cost_per_mtok'), '—');
}

function testPartialRate() {
    const legacy = { ...PERIOD.projects[0] };
    delete legacy.costPerMtokMicros;
    const partial = { ...PERIOD, projects: [{ ...PERIOD.projects[0], partial: true }, legacy] };
    check(
        'project rate is not recomputed',
        projectsTable(partial, 'cost_per_mtok').rows.map(row => row.costPerMtokMicros),
        [8_000_000, null]
    );
}

function testLegendTitle() {
    const spend = PERIOD.providers[1];
    check('legend title 7 days', legendTitle(spend, '7d'), 'Codex · Last 7 days');
    check('legend title 30 days', legendTitle(spend, '30d'), 'Codex · Last 30 days');
}

function testToolsHint() {
    const state = (accounts, spend) => ({ accounts, spend, display: { showSpend: true } });
    const hidden = [{ id: 'a', hidden: true }];
    check('hint under spend', needsToolsHint(state(hidden, {})), true);
    check('no hint with accounts', needsToolsHint(state([{ id: 'a', hidden: false }], {})), false);
    check('no hint when fully empty', needsToolsHint(state([], null)), false);
    check('fully empty', isEmptyState(state([], null)), true);
    check('spend only is not empty', isEmptyState(state([], {})), false);
}

function testMenuWording() {
    const items = starred =>
        cardMenuItems({ providerName: 'X', canHide: false, canStar: true, starred, links: null, canShare: false });
    check('star wording', [items(false)[1].label, items(true)[1].label], ['Always show', 'Show on demand']);
}

function liveFooter(names, intervalSecs) {
    const now = new Date(2026, 8, 25, 12, 26);
    const accounts = names.map(providerName => ({
        hidden: false,
        status: 'fresh',
        providerName,
        refresh: { mode: 'live', intervalSecs },
    }));
    const state = { offline: false, lastSuccessAt: null, nextRefreshAt: null, accounts };
    return footerLines({ kind: 'ready', state }, now, false).second.text;
}

function testRussian() {
    check('ru share comma', preciseShareText(618), '61,8%');
    check('ru live minute', liveFooter(['Claude Code'], 60), 'Вживую — раз в минуту, пока пишет журналы Claude Code');
    check(
        'ru live plural',
        liveFooter(['Claude Code', 'Codex'], 120),
        'Вживую — раз в 2 мин, пока пишут журналы Claude Code, Codex'
    );
    check(
        'ru almost out wording',
        [RU['Used by Almost out'], RU['Will run out and Almost out come through']].every(text =>
            text.includes(`«${RU['Almost out']}»`)
        ),
        true
    );
    check('ru menu wording', [RU['Always show'], RU['Show on demand']], ['Показывать всегда', 'Показывать по запросу']);
}

export function testPopupPolish() {
    testModelUnits();
    testModelTies();
    testProjectUnits();
    testPartialRate();
    testLegendTitle();
    testToolsHint();
    testMenuWording();
    check('en live minute', liveFooter(['Claude'], 60), 'Live — every minute while Claude is active');
    check('en share dot', preciseShareText(618), '61.8%');
    setLanguage('ru');
    try {
        testRussian();
    } finally {
        setLanguage('en');
    }
}
