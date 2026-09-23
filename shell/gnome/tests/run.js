import GLib from 'gi://GLib';
import * as format from '../src/format.js';
import { parseDisplay } from '../src/settings.js';
import { parseState, StateError } from '../src/state.js';
import { check, failures, throws } from './check.js';
import { testSettings, testSettingsUpdates } from './settingsTests.js';
import { testModelBreakdown, testOrder, testProgress } from './shapingTests.js';
import { testExactReset, testForecast } from './timeTests.js';

function readRelative(...parts) {
    const [testFile] = GLib.filename_from_uri(import.meta.url);
    const path = GLib.build_filenamev([GLib.path_get_dirname(testFile), '..', ...parts]);
    const [, bytes] = GLib.file_get_contents(path);
    return new TextDecoder().decode(bytes);
}

const readSample = () => readRelative('dev', 'sample-state.json');
const readDaemonSnapshot = () =>
    readRelative('..', '..', 'crates', 'headroom-daemon', 'src', 'state', 'snapshots', 'state_full.json');

function isPlainObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function shapeMismatches(sample, daemon, path) {
    if (Array.isArray(sample) && Array.isArray(daemon)) {
        if (sample.length === 0 || daemon.length === 0) return [];
        return shapeMismatches(sample[0], daemon[0], `${path}[0]`);
    }
    if (!isPlainObject(sample) || !isPlainObject(daemon)) return [];
    const missing = Object.keys(daemon).filter(key => !(key in sample));
    if (missing.length > 0) return [`${path}: sample lacks daemon keys ${missing.sort()}`];
    return Object.keys(daemon).flatMap(key => shapeMismatches(sample[key], daemon[key], `${path}.${key}`));
}

function testFormat() {
    const now = new Date('2026-09-23T10:00:00Z');
    check('percentLeft', format.percentLeft(61.6), '62% left');
    check('percentLeft null', format.percentLeft(null), '—');
    check('reset hours', format.resetText(new Date('2026-09-23T12:41:00Z'), now), 'Resets in 2h 41m');
    check('reset days', format.resetText(new Date('2026-09-27T16:30:00Z'), now), 'Resets in 4d 6h');
    check('reset soon', format.resetText(new Date('2026-09-23T10:00:30Z'), now), 'Resets soon');
    check('not started', format.resetText(null, now), 'Not started');
    check('spare', format.spareText(4.2), '~4% spare');
    check('percent used', format.percentUsed(38.4), '38% used');
    check('reading left', format.reading({ usedPercent: 38, remainingPercent: 62 }, 'left'), '62% left');
    check('reading used', format.reading({ usedPercent: 38, remainingPercent: 62 }, 'used'), '38% used');
    check('short session', format.shortWindowLabel('session', 'Session'), 'S');
    check('short weekly', format.shortWindowLabel('weekly', 'Weekly'), 'W');
    check('short model', format.shortWindowLabel('model:opus', 'Opus'), 'Opus');
    check('limit', format.limitText(new Date('2026-09-24T19:00:00Z'), now), 'Limit in 1d 9h');
    check('next update', format.nextUpdateText(new Date('2026-09-23T10:03:10Z'), now), 'Next update in 3m');
    check('next update soon', format.nextUpdateText(new Date('2026-09-23T10:00:40Z'), now), 'Next update in <1m');
    check('tokens K', format.compactTokens(1200), '1.2K');
    check('tokens M', format.compactTokens(35_812_904), '35.8M');
    check('tokens B', format.compactTokens(1_500_000_000), '1.5B');
    check('tokens small', format.compactTokens(999), '999');
    check('tokens round', format.compactTokens(3_000_000), '3M');
    check('usd', format.usd(14_370_000), '$14.37');
    check('usd K', format.usd(2_064_000_000), '$2.06K');
    check('usd cents', format.usd(4_050_000), '$4.05');
    check('exact usd', format.exactUsd(1_234_567_890), '$1,234.57');
    check('ring usd', format.ringUsd(463_120_000), '$463');
    check('ring usd small', format.ringUsd(18_420_000), '$18.42');
    check('spend line', format.spendLine({ costMicros: 4_080_000, totalTokens: 1_203_448 }), '$4.08 · 1.2M tokens');
    check('spend empty', format.spendLine({ costMicros: 0, totalTokens: 0 }), 'No data');
}

function testSampleAccounts() {
    const state = parseState(readSample());
    check('accounts', state.accounts.length, 4);
    check('headline', state.headline, {
        accountId: 'codex:1a2b3c4d5e6f',
        windowId: 'session',
        provider: 'codex',
        accountLabel: 'work',
        windowLabel: 'Session',
        usedPercent: 38,
        remainingPercent: 62,
        tone: 'good',
    });
    check('next refresh', state.nextRefreshAt.toISOString(), '2026-09-23T10:03:10.000Z');
    check('last success', state.lastSuccessAt.toISOString(), '2026-09-23T09:58:00.000Z');
    check('online', state.offline, false);
    const personal = state.accounts[1];
    check('stale status', personal.status, 'stale');
    check('window tone', personal.windows[1].tone, 'critical');
    check('pace severity', personal.windows[1].pace.severity, 'running_out');
    check('no spare when running out', personal.windows[1].pace.sparePercent, null);
    check('spare when close', personal.windows[0].pace.sparePercent, 4);
    check('model window id', personal.windows[2].id, 'model:spark');
    check('runs out', personal.windows[1].pace.runsOutAt.toISOString(), '2026-09-24T19:00:00.000Z');
    check('usd balance', state.accounts[0].balances[0], {
        id: 'credits',
        label: 'Credits',
        kind: 'usd',
        usdMicros: 12_500_000,
        value: null,
        unit: null,
    });
    check('count balance', state.accounts[2].balances[0].kind, 'count');
    check('count unit', state.accounts[2].balances[0].unit, 'requests');
    check('error', state.accounts[2].error, {
        kind: 'invalid_response',
        message: 'invalid response: HTTP 503 from api.anthropic.com',
    });
    check('signed out', state.accounts[3].status, 'signed_out');
}

function testSampleContract() {
    const state = parseState(readSample());
    check('display', state.display, parseDisplay(null));
    check(
        'owners',
        state.accounts.map(account => account.owner),
        ['cli', 'headroom', 'cli', 'cli']
    );
    check('window used', state.accounts[0].windows[0].usedPercent, 38);
    check('window hidden', state.accounts[0].windows[0].hidden, false);
    check('own home without logs', state.accounts[1].usage, null);
    check('shared usage', state.accounts[3].usage.provider, 'claude');
    check('hidden flag', state.accounts[0].hidden, false);
}

function testSampleUsage() {
    const state = parseState(readSample());
    const claudeMonth = state.accounts[2].usage.last30Days;
    check('usage tokens total', claudeMonth.tokens.total, 35_812_904);
    check(
        'unpriced',
        [claudeMonth.partial, claudeMonth.unpricedTokens, claudeMonth.unpricedModels],
        [true, 412_000, ['claude-next']]
    );
    check('spend today', state.spend.today.costMicros, 18_420_000);
    check(
        'spend providers',
        state.spend.today.providers.map(spend => spend.provider),
        ['codex', 'claude']
    );
    check('partial month', state.spend.last30Days.partial, true);
    const claudeModels = state.spend.last30Days.providers[1].models;
    check('provider models sorted', claudeModels[0].model, 'claude-opus-4-5');
    check('unpriced model', claudeModels[claudeModels.length - 1], {
        model: 'claude-next',
        totalTokens: 412_000,
        costMicros: 0,
        partial: true,
    });
    check('totals models', claudeMonth.models.length, 7);
    const summed = claudeMonth.models.reduce((sum, entry) => sum + entry.totalTokens, 0);
    check('models add up', summed, claudeMonth.tokens.total);
}

function testDaemonSnapshot() {
    const daemonJson = readDaemonSnapshot();
    const state = parseState(daemonJson);
    check('daemon spend today', state.spend.today.costMicros, 12_400);
    check(
        'daemon spend order',
        state.spend.today.providers.map(spend => [spend.provider, spend.costMicros]),
        [
            ['claude', 10_000],
            ['codex', 2_400],
        ]
    );
    check('daemon partial yesterday', state.spend.yesterday.partial, true);
    check('daemon spare', state.accounts[0].windows[1].pace.sparePercent, 47.5);
    check('daemon error kind', state.accounts[1].error.kind, 'sign_in_expired');
    check('daemon hidden', state.accounts[2].hidden, true);
    check('daemon next refresh', state.nextRefreshAt.toISOString(), '2026-09-23T10:03:00.000Z');
    check('sample shape', shapeMismatches(JSON.parse(readSample()), JSON.parse(daemonJson), '$'), []);
}

function testEdgeStates() {
    throws('bad json', () => parseState('{'), StateError);
    throws('wrong version', () => parseState('{"version": 2}'), StateError);
    const bare = parseState('{"version": 1}');
    check('empty', bare.accounts, []);
    check('no spend without usage', bare.spend, null);
    check('no refresh', bare.nextRefreshAt, null);
    const offline = parseState('{"version": 1, "offline": true, "last_success_at": "2026-09-23T09:13:00Z"}');
    check('offline', [offline.offline, offline.lastSuccessAt.toISOString()], [true, '2026-09-23T09:13:00.000Z']);
}

testFormat();
testExactReset();
testForecast();
testSettings();
testSettingsUpdates();
testModelBreakdown();
testOrder();
testProgress();
testSampleAccounts();
testSampleContract();
testSampleUsage();
testDaemonSnapshot();
testEdgeStates();
if (failures.length > 0) {
    printerr(failures.join('\n'));
    imports.system.exit(1);
}
print('all checks passed');
