import { breakdownValue, modelsTable } from '../src/breakdown.js';
import { centerTextRoom, donutGeometry, fitCenterText, valueReach } from '../src/donutGeometry.js';
import { forecastText } from '../src/format.js';
import { C_, setLanguage } from '../src/i18n.js';
import { loginArgv } from '../src/popup/loginLauncher.js';
import { sheenClip, sheenOffset, sheenStrength, showsSheen } from '../src/popup/sheenPlan.js';
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
}

function rest(name, costMicros, totalTokens, costPerMtokMicros) {
    return { model: name, costMicros, totalTokens, partial: false, costPerMtokMicros };
}

function spendWith(models, modelsOther) {
    const costMicros = models.reduce((sum, model) => sum + model.costMicros, 0) + (modelsOther?.costMicros ?? 0);
    const totalTokens = models.reduce((sum, model) => sum + model.totalTokens, 0) + (modelsOther?.totalTokens ?? 0);
    return {
        provider: 'claude',
        providerName: 'Claude',
        costMicros,
        totalTokens,
        models,
        modelsOther,
    };
}

function period(providers) {
    return {
        costMicros: providers.reduce((sum, spend) => sum + spend.costMicros, 0),
        totalTokens: providers.reduce((sum, spend) => sum + spend.totalTokens, 0),
        partial: false,
        costPerMtokMicros: null,
        providers,
        projects: null,
        projectsOther: null,
    };
}

function testOtherRate() {
    const top = [1, 2, 3, 4, 5].map(index => rest(`m${index}`, index * 10_000_000, index * 1_000_000, 10_000_000));
    const folded = {
        count: 3,
        costMicros: 900_000,
        totalTokens: 600_000,
        partial: false,
        costPerMtokMicros: 1_500_000,
    };
    const single = modelsTable(period([spendWith(top, folded)]), 'cost_per_mtok');
    check('other rate from the daemon', breakdownValue(single.rows.at(-1), 'cost_per_mtok'), '$1.50');
    const unrated = modelsTable(period([spendWith(top, { ...folded, costPerMtokMicros: null })]), 'cost_per_mtok');
    check('other without a daemon rate', breakdownValue(unrated.rows.at(-1), 'cost_per_mtok'), '—');
    const extra = [...top, rest('m6', 100_000, 50_000, 2_000_000)];
    const mixed = modelsTable(period([spendWith(extra, folded)]), 'cost_per_mtok');
    check('folded pieces show no rate', mixed.rows.at(-1).costPerMtokMicros, null);
    const lone = modelsTable(period([spendWith(extra, null)]), 'cost_per_mtok');
    check('one folded model keeps its rate', breakdownValue(lone.rows.at(-1), 'cost_per_mtok'), '$2.00');
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
    testOtherRate();
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
