import GLib from 'gi://GLib';
import { parseDisplay } from '../src/settings.js';
import { parseState, StateError } from '../src/state.js';
import { check, failures, throws } from './check.js';
import { testCombined, testCombinedSnapshot } from './combinedTests.js';
import { testDonut } from './donutTests.js';
import { testFormat, testNumbers } from './formatTests.js';
import { testLocale } from './localeTests.js';
import { testMoney } from './moneyTests.js';
import { testNotices } from './noticeTests.js';
import { testOptionModel } from './optionModelTests.js';
import { testPayload, testSampleAdditions } from './payloadTests.js';
import { testPrefsClient } from './prefsClientTests.js';
import { testProgressProcess } from './processTests.js';
import { testSerialQueue } from './queueTests.js';
import { testRefresh } from './refreshTests.js';
import { testSettings06 } from './settings06Tests.js';
import { testSettings, testSettingsUpdates } from './settingsTests.js';
import { testProviders } from './providerTests.js';
import { testModelBreakdown, testOrder, testProgress } from './shapingTests.js';
import { testSpendUnits } from './spendUnitTests.js';
import { testStatus } from './statusTests.js';
import { testClockFormats, testExactReset, testForecast } from './timeTests.js';
import { testUpdateCheck } from './updateCheckTests.js';
import { testUpdate } from './updateTests.js';

function readRelative(...parts) {
    const [testFile] = GLib.filename_from_uri(import.meta.url);
    const path = GLib.build_filenamev([GLib.path_get_dirname(testFile), '..', ...parts]);
    const [, bytes] = GLib.file_get_contents(path);
    return new TextDecoder().decode(bytes);
}

const readSample = () => readRelative('dev', 'sample-state.json');
const readSnapshot = name => readRelative('..', '..', 'crates', 'headroom-daemon', 'src', 'state', 'snapshots', name);
const readDaemonSnapshot = () => readSnapshot('state_full.json');

function isPlainObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

const DICTIONARY_PATHS = new Set(['$.display.hidden_windows']);

function firstValue(dictionary) {
    return Object.values(dictionary)[0];
}

function shapeMismatches(sample, daemon, path) {
    if (Array.isArray(sample) && Array.isArray(daemon)) {
        if (sample.length === 0 || daemon.length === 0) return [];
        return shapeMismatches(sample[0], daemon[0], `${path}[0]`);
    }
    if (!isPlainObject(sample) || !isPlainObject(daemon)) return [];
    if (DICTIONARY_PATHS.has(path)) return shapeMismatches(firstValue(sample), firstValue(daemon), `${path}[*]`);
    const missing = Object.keys(daemon).filter(key => !(key in sample));
    if (missing.length > 0) return [`${path}: sample lacks daemon keys ${missing.sort()}`];
    return Object.keys(daemon).flatMap(key => shapeMismatches(sample[key], daemon[key], `${path}.${key}`));
}

function testSampleHeadline() {
    const state = parseState(readSample());
    check('headline', state.headline, {
        accountId: 'codex:1a2b3c4d5e6f',
        windowId: 'session',
        provider: 'codex',
        providerName: 'Codex',
        accountLabel: 'work',
        windowLabel: 'Session',
        usedPercent: 38,
        remainingPercent: 62,
        tone: 'good',
        combined: false,
        accountCount: 1,
    });
}

function testSampleAccounts() {
    const state = parseState(readSample());
    check('accounts', state.accounts.length, 6);
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
        currency: null,
        micros: null,
        value: null,
        unit: null,
    });
    check('count balance', state.accounts[2].balances[0].kind, 'count');
    check('count unit', state.accounts[2].balances[0].unit, 'requests');
    check('error', state.accounts[2].error, {
        kind: 'invalid_response',
        message: 'invalid response: HTTP 503 from api.anthropic.com',
    });
}

function testSampleContract() {
    const state = parseState(readSample());
    check('display', state.display, parseDisplay(null));
    check(
        'owners',
        state.accounts.map(account => account.owner),
        ['cli', 'headroom', 'cli', 'cli', 'headroom', 'headroom']
    );
    check('window used', state.accounts[0].windows[0].usedPercent, 38);
    check('window hidden', state.accounts[0].windows[0].hidden, false);
    check('own home without logs', state.accounts[1].usage, null);
    check('shared usage', state.accounts[3].usage.provider, 'claude');
    check('hidden flag', state.accounts[0].hidden, false);
    check('signed out', state.accounts[3].status, 'signed_out');
    check(
        'recoveries',
        state.accounts.map(account => account.recovery?.action ?? null),
        [null, null, 'retry', 'retry', null, null]
    );
    check('update check', state.updateCheck.checkedAt.toISOString(), '2026-09-23T04:00:00.000Z');
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
    check('spend today', state.spend.today.costMicros, 19_730_000);
    check(
        'spend providers',
        state.spend.today.providers.map(spend => [spend.provider, spend.providerName]),
        [
            ['codex', 'Codex'],
            ['claude', 'Claude'],
            ['opencode', 'OpenCode'],
        ]
    );
    check('partial month', state.spend.last30Days.partial, true);
    const claudeModels = state.spend.last30Days.providers[1].models;
    check('provider models sorted', claudeModels[0].model, 'claude-opus-4-5');
    check('provider other', state.spend.last30Days.providers[1].modelsOther, {
        count: 2,
        totalTokens: 2_182_045,
        costMicros: 964_000,
        partial: true,
    });
    check('totals models', claudeMonth.models.length, 5);
    const summed = claudeMonth.models.reduce((sum, entry) => sum + entry.totalTokens, 0);
    check('models add up', summed + claudeMonth.modelsOther.totalTokens, claudeMonth.tokens.total);
    check('no other when few', state.accounts[0].usage.last30Days.modelsOther, null);
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
testNumbers();
testMoney();
testLocale();
testExactReset();
testForecast();
testClockFormats();
testSettings();
testSettings06();
testPayload();
testSpendUnits();
testSettingsUpdates();
testModelBreakdown();
testOrder();
testProgress();
testSampleHeadline();
testSampleAccounts();
testProviders(parseState(readSample()));
testSampleContract();
testSampleUsage();
testSampleAdditions(parseState(readSample()));
testDaemonSnapshot();
testEdgeStates();
testStatus();
testNotices();
testDonut();
testCombined();
testCombinedSnapshot(readSnapshot('state_combined.json'));
testRefresh();
testUpdate();
testUpdateCheck();
testOptionModel();
await testSerialQueue();
await testProgressProcess();
await testPrefsClient();
if (failures.length > 0) {
    printerr(failures.join('\n'));
    imports.system.exit(1);
}
print('all checks passed');
