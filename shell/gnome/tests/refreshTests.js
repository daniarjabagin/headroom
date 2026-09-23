import { ERROR_MS, MIN_SPIN_MS, RefreshTracker } from '../src/refreshTracker.js';
import { EASE_MS, stopPlan, TURN_MS } from '../src/spin.js';
import { check } from './check.js';

function testStopPlan() {
    check('ease matches cruise speed', EASE_MS, TURN_MS / 2);
    check('at rest', stopPlan(0), null);
    check('full turns rest', stopPlan(720), null);
    check('half turn', stopPlan(180), { from: 180, cruiseTo: 270, cruiseMs: 350, target: 360 });
    check('quarter left', stopPlan(270), { from: 270, cruiseTo: 270, cruiseMs: 0, target: 360 });
    check('extra turn when close', stopPlan(300), { from: 300, cruiseTo: 630, cruiseMs: 1283, target: 720 });
    check('wraps accumulated angle', stopPlan(450), { from: 90, cruiseTo: 270, cruiseMs: 700, target: 360 });
}

function testTrackerPress() {
    const tracker = new RefreshTracker();
    check('idle', tracker.mode(0), 'idle');
    check('first press calls', tracker.press(1000), true);
    check('busy while in flight', tracker.mode(1000), 'busy');
    check('coalesces second press', tracker.press(1100), false);
    tracker.settle(true, 1200);
    check('holds minimum spin', tracker.mode(1200), 'busy');
    check('next change at hold end', tracker.nextChange(1200), 1000 + MIN_SPIN_MS);
    check('coalesces during hold', tracker.press(1500), false);
    check('idle after hold', tracker.mode(1000 + MIN_SPIN_MS), 'idle');
    check('no pending change', tracker.nextChange(1000 + MIN_SPIN_MS), null);
}

function testTrackerDaemon() {
    const tracker = new RefreshTracker();
    tracker.press(0);
    tracker.settle(true, 10);
    tracker.setDaemonBusy(true);
    check('spins while daemon refreshes', tracker.mode(5000), 'busy');
    tracker.setDaemonBusy(false);
    check('stops when fresh', tracker.mode(5000), 'idle');
    check('presses again after', tracker.press(5000), true);
}

function testTrackerFailure() {
    const tracker = new RefreshTracker();
    tracker.press(0);
    tracker.settle(false, 100);
    check('error', tracker.mode(100), 'error');
    check('error ends', tracker.nextChange(100), 100 + ERROR_MS);
    check('idle after error', tracker.mode(100 + ERROR_MS), 'idle');
    check('retry during error', new RefreshTracker().press(0), true);
    tracker.press(200);
    check('press clears error', tracker.mode(200), 'busy');
}

export function testRefresh() {
    testStopPlan();
    testTrackerPress();
    testTrackerDaemon();
    testTrackerFailure();
}
