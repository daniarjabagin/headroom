import { barParts, breakdownValue, modelsTable } from '../src/breakdown.js';
import { centerTextRoom, donutGeometry, fitCenterText, valueReach } from '../src/donutGeometry.js';
import { forecastText } from '../src/format.js';
import { C_, setLanguage } from '../src/i18n.js';
import { loginArgv } from '../src/popup/loginLauncher.js';
import {
    pointerQuiet,
    QUIET_MS,
    sheenClip,
    sheenOffset,
    sheenStrength,
    showsSheen,
    STEP_MS,
    SWEEP_MS,
    sweepProgress,
} from '../src/popup/sheenPlan.js';
import { paceNote } from '../src/quota.js';
import { parseDisplay } from '../src/settings.js';
import { check } from './check.js';

const FIT = { minSize: 7.5, step: 0.25 };

function testCenterRoom() {
    const inner = donutGeometry(104).inner;
    check('room at the center line', Math.round(centerTextRoom(104, 0, 4) * 100), Math.round((2 * inner - 8) * 100));
    check('room shrinks away from the center', centerTextRoom(104, 16, 4) < centerTextRoom(104, 8, 4), true);
    check('no room past the hole', centerTextRoom(104, 80, 4), 0);
    check('padding never goes negative', centerTextRoom(10, 0, 40), 0);
}

function testCenterFit() {
    check('fits at the base size', fitCenterText(40, 11.25, 56, FIT), { size: 11.25, clip: false });
    check('shrinks to fit', fitCenterText(70, 11.25, 56, FIT), { size: 9, clip: false });
    const shrunk = fitCenterText(70, 11.25, 56, FIT);
    check('shrunk text fits the room', (70 * shrunk.size) / 11.25 <= 56, true);
    check('stops at the minimum', fitCenterText(140, 11.25, 56, FIT), { size: 7.5, clip: true });
    check('steps down in quarter points', (fitCenterText(61, 11.25, 56, FIT).size * 4) % 1, 0);
    check('reach of a lone value', valueReach({ lineHeight: 20, captionHeight: 0, inkY: 5, inkHeight: 11 }), 6);
    check('reach with a caption', valueReach({ lineHeight: 20, captionHeight: 12, inkY: 5, inkHeight: 11 }), 11);
}

function testSheen() {
    check('sheen on a healthy meter', showsSheen(0.62, 'ok'), true);
    check('sheen on warning and critical tones', [showsSheen(0.5, 'warn'), showsSheen(0.9, 'crit')], [true, true]);
    check('no sheen under 3 %', showsSheen(0.02, 'ok'), false);
    check('no sheen without data', showsSheen(0.5, 'none'), false);
    check('clip keeps off the round caps', sheenClip(10, 100, 5), { x: 12.5, width: 95 });
    check('clip of an empty fill', sheenClip(10, 0, 5).width, 0);
    check(
        'band sweeps from outside to outside',
        [0, 0.5, 1].map(progress => sheenOffset(progress, 200, 40)),
        [-40, 80, 200]
    );
    check(
        'band fades in and out at the clip edges',
        [-40, -20, 0, 80, 180, 200].map(offset => sheenStrength(offset, 200, 40)),
        [0, 0.5, 1, 1, 0.5, 0]
    );
    check('short fill dims the band', sheenStrength(0, 20, 40), 0.5);
    check(
        'sweep eases in and out',
        [-100, 0, SWEEP_MS / 2, SWEEP_MS, SWEEP_MS * 2].map(
            elapsed => Math.round(sweepProgress(elapsed) * 1000) / 1000
        ),
        [0, 0, 0.5, 1, 1]
    );
    check('sweep steps stay near 30 fps', Math.round(SWEEP_MS / STEP_MS), 42);
    check('pointer busy holds the sheen', [pointerQuiet(200), pointerQuiet(QUIET_MS)], [false, true]);
}

function ranked(provider, name, costMicros, totalTokens, costPerMtokMicros) {
    return {
        provider,
        providerName: provider,
        model: name,
        costMicros,
        totalTokens,
        partial: false,
        costPerMtokMicros,
    };
}

const DAEMON_MODELS = [
    ranked('claude', 'opus', 40_000_000, 8_000_000, 5_000_000),
    ranked('codex', 'gpt-5', 30_000_000, 20_000_000, 1_500_000),
    ranked('claude', 'sonnet', 20_000_000, 12_000_000, 1_666_667),
    ranked('codex', 'mini', 5_000_000, 5_000_000, 1_000_000),
    ranked('codex', 'nano', 3_000_000, 3_000_000, 1_000_000),
];

function daemonPeriod(modelsOther) {
    return {
        costMicros: 100_000_000,
        totalTokens: 50_000_000,
        partial: false,
        costPerMtokMicros: 2_000_000,
        providers: [],
        models: DAEMON_MODELS,
        modelsOther,
        projects: null,
        projectsOther: null,
    };
}

const FOLDED = { count: 3, costMicros: 2_000_000, totalTokens: 2_000_000, partial: true, costPerMtokMicros: 1_250_000 };

function testDaemonModels() {
    const period = daemonPeriod(FOLDED);
    const table = modelsTable(period, 'cost');
    check(
        'daemon models keep the daemon order',
        table.rows.map(row => [row.name, row.parts[0]?.series ?? null]),
        [
            ['opus', 'claude'],
            ['gpt-5', 'codex'],
            ['sonnet', 'claude'],
            ['mini', 'codex'],
            ['nano', 'codex'],
            ['Other', null],
        ]
    );
    check('daemon model count', table.count, 8);
    check('other bar is neutral', barParts(table.rows[5], period, 'cost'), [{ series: null, permille: 20 }]);
    const rates = modelsTable(period, 'cost_per_mtok').rows.map(row => breakdownValue(row, 'cost_per_mtok'));
    check('other rate from the daemon', rates, ['$1.50', '$1.67', '$5.00', '$1.00', '$1.00', '$1.25']);
    const unrated = modelsTable(daemonPeriod({ ...FOLDED, costPerMtokMicros: null }), 'cost_per_mtok');
    check('other without a daemon rate', breakdownValue(unrated.rows.at(-1), 'cost_per_mtok'), '—');
    check(
        'tokens reorder the listed models only',
        modelsTable(period, 'tokens').rows.map(row => row.name),
        ['gpt-5', 'sonnet', 'opus', 'mini', 'nano', 'Other']
    );
    check('no other when nothing folded', modelsTable(daemonPeriod(null), 'cost').rows.length, 5);
}

function testSampleModels(sample) {
    for (const key of ['today', 'yesterday', 'last7Days', 'last30Days']) {
        const period = sample.spend[key];
        const rows = modelsTable(period, 'cost').rows;
        const cost = rows.reduce((sum, row) => sum + row.costMicros, 0);
        const tokens = rows.reduce((sum, row) => sum + row.totalTokens, 0);
        check(`sample ${key} models reconcile`, [cost, tokens], [period.costMicros, period.totalTokens]);
    }
    check('sample other has a rate', sample.spend.last30Days.modelsOther.costPerMtokMicros > 0, true);
}

function testPausedNote() {
    const pace = { severity: 'running_out', sparePercent: null, runsOutAt: null, basis: 'paused' };
    check('paused over pace without forecast', paceNote({ pace }, new Date(), false), {
        flame: true,
        text: 'Over pace',
    });
    const counting = { ...pace, basis: 'recent' };
    check('running out without forecast', paceNote({ pace: counting }, new Date(), false).text, 'Limit soon');
}

function testBreakdownSetting() {
    check('breakdown shown by default', parseDisplay(null).showBreakdown, true);
    check('breakdown hidden', parseDisplay({ show_breakdown: false }).showBreakdown, false);
    check('breakdown ignores junk', parseDisplay({ show_breakdown: 'no' }).showBreakdown, true);
}

function testSample(sample) {
    const weekly = sample.accounts[0].windows[1];
    check('sample paused basis', [weekly.pace.basis, weekly.pace.activeLeftSeconds], ['paused', 11_400]);
    check('sample window basis', sample.accounts[0].windows[0].pace.basis, 'window');
    check('sample untracked basis', sample.accounts[2].windows[0].pace.basis, null);
    const display = { valueMode: 'left', resetFormat: 'countdown' };
    check('sample cli sign in', sample.accounts[3].recovery.accountId, 'claude:5e4d3c2b1a0f');
    check('sample paused forecast', forecastText(weekly, new Date(), display), 'Paused · lasts ≈3h of work');
}

function testRussian() {
    setLanguage('ru');
    const pace = { severity: 'healthy', sparePercent: 40, basis: 'paused', activeLeftSeconds: 3 * 3600 };
    const window = { remainingPercent: 60, resetsAt: null, pace };
    const display = { valueMode: 'left', resetFormat: 'countdown' };
    check('ru paused forecast', forecastText(window, new Date(), display), 'Пауза · хватит ≈3 ч работы');
    const idle = { ...window, pace: { ...pace, activeLeftSeconds: null } };
    check('ru paused', forecastText(idle, new Date(), display), 'Пауза');
    check('ru sign in button', C_('button', 'Sign in'), 'Войти');
    setLanguage('en');
    check('en sign in button', C_('button', 'Sign in'), 'Sign in');
}

export function testPopupFit(sample) {
    testCenterRoom();
    testCenterFit();
    testSheen();
    testDaemonModels();
    testSampleModels(sample);
    testPausedNote();
    testBreakdownSetting();
    testSample(sample);
    testRussian();
    check('login command', loginArgv('/usr/bin/headroom', 'claude:1'), [
        '/usr/bin/headroom',
        'accounts',
        'login',
        'claude:1',
    ]);
}
