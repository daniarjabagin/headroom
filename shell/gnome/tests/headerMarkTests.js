import { headerStatusKind, incidentTip } from '../src/popup/headerMarks.js';
import { check } from './check.js';

const INCIDENT = { provider: 'claude', indicator: 'critical', tone: 'critical', title: 'API down' };
const ONLINE = { offline: false };

function account(status, error = null) {
    return { status, error };
}

export function testHeaderMarks() {
    const failed = account('error', { kind: 'http', message: 'HTTP 500' });
    check('error alone keeps the triangle', headerStatusKind(ONLINE, failed, null), 'error');
    check('incident replaces the triangle', headerStatusKind(ONLINE, failed, INCIDENT), null);
    check('refreshing stays', headerStatusKind(ONLINE, account('refreshing'), INCIDENT), 'refreshing');
    check('outdated stays', headerStatusKind(ONLINE, account('stale'), INCIDENT), 'outdated');
    check(
        'offline failure is outdated',
        headerStatusKind({ offline: true }, account('error', { kind: 'network', message: 'x' }), INCIDENT),
        'outdated'
    );
    check('fresh has no mark', headerStatusKind(ONLINE, account('fresh'), null), null);
    check('tip adds the error', incidentTip(ONLINE, INCIDENT, failed), 'Major outage · API down\nHTTP 500');
    check(
        'tip without message',
        incidentTip(ONLINE, INCIDENT, account('error')),
        'Major outage · API down\nRefresh failed'
    );
    check('tip alone', incidentTip(ONLINE, INCIDENT, account('fresh')), 'Major outage · API down');
}
