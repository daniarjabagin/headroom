import * as format from '../src/format.js';
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
}
