import { clockTime, usesHour12 } from '../src/dates.js';
import * as format from '../src/format.js';
import { setLanguage } from '../src/i18n.js';
import { check } from './check.js';

export function testExactReset() {
    const now = new Date(2026, 8, 23, 10, 0);
    const at = (day, hour, minute) => new Date(2026, 8, day, hour, minute);
    check('exact today', format.resetText(at(23, 14, 30), now, 'exact'), 'Resets today at 14:30');
    check('exact tomorrow', format.resetText(at(24, 9, 5), now, 'exact'), 'Resets tomorrow at 09:05');
    check('exact weekday', format.resetText(at(26, 18, 0), now, 'exact'), 'Resets Sat at 18:00');
    check('exact far', format.resetText(new Date(2026, 9, 2, 8, 0), now, 'exact'), 'Resets Oct 2 at 08:00');
    check('countdown default', format.resetText(at(23, 12, 41), now), 'Resets in 2h 41m');
    check('exact not started', format.resetText(null, now, 'exact'), 'Not started');
    check('exact past', format.resetText(at(23, 9, 30), now, 'exact'), 'Reset pending');
    check('exact past yesterday', format.resetText(at(22, 23, 0), now, 'exact'), 'Reset pending');
    check('countdown past', format.resetText(at(23, 9, 59), now), 'Reset pending');
    check('exact now', format.resetText(now, now, 'exact'), 'Reset pending');
}

function paceWindow(pace, resetsAt) {
    return {
        usedPercent: 71,
        remainingPercent: 29,
        resetsAt,
        pace: { evenPacePercent: 50, projectedPercent: null, sparePercent: null, runsOutAt: null, ...pace },
    };
}

const LEFT = { valueMode: 'left', resetFormat: 'countdown' };

function testRunOutForecast(now) {
    const resets = new Date('2026-09-23T12:28:00Z');
    const runsOutAt = new Date('2026-09-23T10:45:00Z');
    const runningOut = paceWindow({ severity: 'running_out', runsOutAt }, resets);
    check(
        'forecast run out',
        format.forecastText(runningOut, now, LEFT),
        'At this pace: runs out in 45m · resets in 2h 28m'
    );
    const noReset = paceWindow({ severity: 'running_out', runsOutAt }, null);
    check('forecast run out no reset', format.forecastText(noReset, now, LEFT), 'At this pace: runs out in 45m');
    const exact = { valueMode: 'left', resetFormat: 'exact' };
    const local = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 23, 59);
    const sameDay = paceWindow({ severity: 'running_out', runsOutAt: new Date(now.getTime() + 60_000 * 45) }, local);
    check('forecast exact reset', format.forecastText(sameDay, now, exact).endsWith('resets today at 23:59'), true);
    const overdue = paceWindow({ severity: 'running_out', runsOutAt: new Date('2026-09-23T09:00:00Z') }, resets);
    check('forecast overdue', format.forecastText(overdue, now, LEFT), 'At this pace: runs out any minute');
}

export function testForecast() {
    const now = new Date('2026-09-23T10:00:00Z');
    const resets = new Date('2026-09-23T12:28:00Z');
    testRunOutForecast(now);
    const healthy = paceWindow({ severity: 'healthy', projectedPercent: 60.2, sparePercent: 39.8 }, resets);
    check('forecast spare', format.forecastText(healthy, now, LEFT), 'At this pace: ~40% left at reset');
    const used = { valueMode: 'used', resetFormat: 'countdown' };
    check('forecast used', format.forecastText(healthy, now, used), 'At this pace: ~60% used at reset');
    const close = paceWindow({ severity: 'close', projectedPercent: 96, sparePercent: 4 }, resets);
    check('forecast close', format.forecastText(close, now, LEFT), 'At this pace: ~4% left at reset');
    check('forecast untracked', format.forecastText(paceWindow({ severity: 'untracked' }, resets), now, LEFT), null);
    check('forecast spent', format.forecastText(paceWindow({ severity: 'spent' }, resets), now, LEFT), null);
    check('forecast no data', format.forecastText({ ...healthy, remainingPercent: null }, now, LEFT), null);
    testPausedForecast(now, resets);
}

function testPausedForecast(now, resets) {
    const paused = paceWindow(
        { severity: 'healthy', sparePercent: 40, basis: 'paused', activeLeftSeconds: 11_400 },
        resets
    );
    check('forecast paused', format.forecastText(paused, now, LEFT), 'Paused · lasts ≈3h of work');
    const unknown = paceWindow({ severity: 'healthy', basis: 'paused', activeLeftSeconds: null }, resets);
    check('forecast paused without pace', format.forecastText(unknown, now, LEFT), 'Paused');
    const recent = paceWindow(
        { severity: 'running_out', basis: 'recent', runsOutAt: new Date('2026-09-23T10:45:00Z') },
        null
    );
    check('forecast recent', format.forecastText(recent, now, LEFT), 'At this pace: runs out in 45m');
    const minute = 60;
    check(
        'rough durations',
        [
            30,
            44 * minute + 40,
            59 * minute + 40,
            150 * minute,
            23 * 60 * minute + 40 * minute,
            28 * 60 * minute + 10 * minute,
        ].map(secs => format.roughDuration(secs * 1000)),
        ['1m', '45m', '1h', '3h', '1d 0h', '1d 4h']
    );
}

function testTwelveHour() {
    const at = (hour, minute) => new Date(2026, 8, 23, hour, minute);
    check(
        '12h clock',
        [at(0, 0), at(0, 5), at(11, 59), at(12, 0), at(13, 7), at(23, 59)].map(date => clockTime(date, true)),
        ['12:00 AM', '12:05 AM', '11:59 AM', '12:00 PM', '1:07 PM', '11:59 PM']
    );
    check(
        '24h clock',
        [at(0, 0), at(12, 0), at(9, 5), at(23, 59)].map(date => clockTime(date)),
        ['00:00', '12:00', '09:05', '23:59']
    );
    const now = at(10, 0);
    check('exact 12h', format.resetText(at(14, 30), now, 'exact', false, true), 'Resets today at 2:30 PM');
    check(
        'exact 12h midnight',
        format.resetText(new Date(2026, 8, 24, 0, 0), now, 'exact', false, true),
        'Resets tomorrow at 12:00 AM'
    );
    check(
        'exact 24h midnight',
        format.resetText(new Date(2026, 8, 24, 0, 0), now, 'exact'),
        'Resets tomorrow at 00:00'
    );
    const running = paceWindow({ severity: 'running_out', runsOutAt: at(10, 45) }, at(12, 0));
    const exact = { valueMode: 'left', resetFormat: 'exact' };
    check(
        'forecast 12h noon',
        format.forecastText(running, now, exact, true),
        'At this pace: runs out in 45m · resets today at 12:00 PM'
    );
    setLanguage('ru');
    try {
        check('ru 12h clock', clockTime(at(15, 4), true), '3:04 PM');
    } finally {
        setLanguage('en');
    }
}

function testHourCycle() {
    check(
        'hour cycle',
        [
            usesHour12('12h', '24h'),
            usesHour12('24h', '12h'),
            usesHour12('auto', '12h'),
            usesHour12('auto', '24h'),
            usesHour12('auto', null),
        ],
        [true, false, true, false, false]
    );
}

export function testClockFormats() {
    testTwelveHour();
    testHourCycle();
}
