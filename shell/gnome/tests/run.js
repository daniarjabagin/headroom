import GLib from 'gi://GLib';
import * as format from '../src/format.js';
import { parseState, StateError } from '../src/state.js';

const failures = [];

function check(name, actual, expected) {
    if (JSON.stringify(actual) !== JSON.stringify(expected))
        failures.push(`${name}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
}

function throws(name, run, type) {
    try {
        run();
        failures.push(`${name}: expected ${type.name}`);
    } catch (error) {
        if (!(error instanceof type)) failures.push(`${name}: threw ${error}`);
    }
}

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
    const sampleKeys = Object.keys(sample).sort();
    const daemonKeys = Object.keys(daemon).sort();
    if (JSON.stringify(sampleKeys) !== JSON.stringify(daemonKeys))
        return [`${path}: sample keys ${sampleKeys} vs daemon keys ${daemonKeys}`];
    return daemonKeys.flatMap(key => shapeMismatches(sample[key], daemon[key], `${path}.${key}`));
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
    check('left at reset', format.leftAtResetText(18), '~18% left at reset');
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
    check('shared usage', personal.usage.provider, 'codex');
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
testSampleAccounts();
testSampleUsage();
testDaemonSnapshot();
testEdgeStates();
if (failures.length > 0) {
    printerr(failures.join('\n'));
    imports.system.exit(1);
}
print('all checks passed');
