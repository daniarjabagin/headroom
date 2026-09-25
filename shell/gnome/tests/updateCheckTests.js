import { setLanguage } from '../src/i18n.js';
import { parseState } from '../src/state.js';
import { checkRow, failedCheck, parseCheckResult, parseUpdateCheck } from '../src/updateCheck.js';
import { check } from './check.js';

const NOW = new Date('2026-09-23T10:05:00Z');
const CHECKED = { checkedAt: new Date('2026-09-23T10:00:00Z') };
const UPDATE = { version: '0.7.0', url: null, publishedAt: null, install: 'unknown', command: null };

function row(overrides = {}) {
    return checkRow({
        enabled: true,
        appVersion: '0.6.0',
        update: null,
        updateCheck: CHECKED,
        result: null,
        checking: false,
        now: NOW,
        ...overrides,
    });
}

function testParsing() {
    check('no update check', parseUpdateCheck(undefined), null);
    check('never checked', parseUpdateCheck({ checked_at: null }), { checkedAt: null });
    const state = parseState(
        '{"version": 1, "app_version": "0.6.0", "update_check": {"checked_at": "2026-09-23T04:00:00Z"}}'
    );
    check('state app version', state.appVersion, '0.6.0');
    check('state checked at', state.updateCheck.checkedAt.toISOString(), '2026-09-23T04:00:00.000Z');
    const older = parseState('{"version": 1}');
    check('older daemon', [older.appVersion, older.updateCheck], [null, null]);
    const limited = parseCheckResult(
        '{"status":"rate_limited","checked_at":"2026-09-22T09:14:00Z","version":"0.5.1","until":"2026-09-23T11:30:00Z"}'
    );
    check(
        'rate limited',
        [limited.status, limited.version, limited.until.toISOString()],
        ['rate_limited', '0.5.1', '2026-09-23T11:30:00.000Z']
    );
    check('unknown status fails', parseCheckResult('{"status":"maybe"}').status, 'failed');
    check('garbage fails', parseCheckResult('{').status, 'failed');
    check('not supported', failedCheck('org.freedesktop.DBus.Error.NotSupported').status, 'unsupported');
    check('unknown method', failedCheck('org.freedesktop.DBus.Error.UnknownMethod').status, 'unsupported');
    check('timeout fails', failedCheck('org.freedesktop.DBus.Error.NoReply').status, 'failed');
    check('local error fails', failedCheck(null).status, 'failed');
}

function testHidden() {
    check('updates off', row({ enabled: false }).visible, false);
    check('no update check in state', row({ updateCheck: null }).visible, false);
    check('release row takes over', row({ update: UPDATE }).visible, false);
    check('method missing', row({ result: failedCheck('org.freedesktop.DBus.Error.UnknownMethod') }).visible, false);
    check('disabled answer', row({ result: parseCheckResult('{"status":"disabled"}') }).visible, false);
}

function texts(overrides) {
    const shown = row(overrides);
    return [shown.title, shown.subtitle, shown.action, shown.busy];
}

function testWording() {
    check('up to date', texts({}), ["You're up to date", 'Headroom 0.6.0 · checked 5m ago', 'Check now', false]);
    const justNow = { checkedAt: new Date('2026-09-23T10:04:40Z') };
    check('checked just now', row({ updateCheck: justNow }).subtitle, 'Headroom 0.6.0 · checked just now');
    check('checking', texts({ checking: true }), [
        'Checking for updates…',
        'Headroom 0.6.0 · checked 5m ago',
        'Check now',
        true,
    ]);
    const available = parseCheckResult('{"status":"available","checked_at":"2026-09-23T10:05:00Z","version":"0.7.0"}');
    check('available before the state catches up', row({ result: available }).title, 'Headroom 0.7.0 is available');
    check('failed', texts({ result: failedCheck(null) }), [
        "Couldn't check for updates",
        'Headroom 0.6.0 · checked 5m ago',
        'Try again',
        false,
    ]);
    const until = new Date(2026, 8, 23, 14, 30);
    const limited = { ...failedCheck(null), status: 'rate_limited', until };
    const beforeUntil = new Date(until.getTime() - 60_000);
    check('rate limited', texts({ result: limited, now: beforeUntil }).slice(0, 3), [
        'GitHub rate limit',
        'Try again at 14:30',
        'Check now',
    ]);
    const afterUntil = new Date(until.getTime() + 60_000);
    check('rate limit over', row({ result: limited, now: afterUntil }).title, "You're up to date");
    check('never checked', texts({ updateCheck: { checkedAt: null } }).slice(0, 2), [
        'Not checked yet',
        'Headroom 0.6.0',
    ]);
}

function testRussianWording() {
    setLanguage('ru');
    try {
        check('ru up to date', texts({}).slice(0, 3), [
            'У вас последняя версия',
            'Headroom 0.6.0 · проверено 5 мин назад',
            'Проверить сейчас',
        ]);
    } finally {
        setLanguage('en');
    }
}

export function testUpdateCheck() {
    testParsing();
    testHidden();
    testWording();
    testRussianWording();
}
