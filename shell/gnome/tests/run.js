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

function readSample() {
    const [testFile] = GLib.filename_from_uri(import.meta.url);
    const path = GLib.build_filenamev([GLib.path_get_dirname(testFile), '..', 'dev', 'sample-state.json']);
    const [, bytes] = GLib.file_get_contents(path);
    return new TextDecoder().decode(bytes);
}

function testFormat() {
    const now = new Date('2026-09-23T10:00:00Z');
    check('percentLeft', format.percentLeft(61.6), '62% left');
    check('percentLeft null', format.percentLeft(null), '—');
    check('reset hours', format.resetText(new Date('2026-09-23T12:41:00Z'), now), 'Resets in 2h 41m');
    check('reset days', format.resetText(new Date('2026-09-27T16:30:00Z'), now), 'Resets in 4d 6h');
    check('reset soon', format.resetText(new Date('2026-09-23T10:00:30Z'), now), 'Resets soon');
    check('not started', format.resetText(null, now), 'Not started');
    check('spare', format.spareText(96), '~4% spare');
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

function testState() {
    const state = parseState(readSample());
    check('accounts', state.accounts.length, 4);
    check('headline', state.headline, {
        accountId: 'codex:1a2b3c4d5e6f',
        windowId: 'session',
        remainingPercent: 62,
        tone: 'good',
    });
    const personal = state.accounts[1];
    check('stale status', personal.status, 'stale');
    check('window tone', personal.windows[1].tone, 'critical');
    check('pace severity', personal.windows[1].pace.severity, 'running_out');
    check('runs out', personal.windows[1].pace.runsOutAt.toISOString(), '2026-09-24T19:00:00.000Z');
    check('shared usage', personal.usage.provider, 'codex');
    check('no data window', state.accounts[2].windows[2].remainingPercent, null);
    check('error message', state.accounts[2].error, 'HTTP 503 from api.anthropic.com');
    check('signed out', state.accounts[3].status, 'signed_out');
    check('spend today', state.spend.today.costMicros, 18_420_000);
    check('spend slices', state.spend.today.slices.length, 2);
    check('partial month', state.spend.month.partial, true);
    throws('bad json', () => parseState('{'), StateError);
    throws('wrong version', () => parseState('{"version": 2}'), StateError);
    check('empty', parseState('{"version": 1}').accounts, []);
}

testFormat();
testState();
if (failures.length > 0) {
    printerr(failures.join('\n'));
    imports.system.exit(1);
}
print('all checks passed');
