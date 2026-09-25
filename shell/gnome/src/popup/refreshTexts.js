import { clockTime } from '../dates.js';
import { agoText, duration, nextUpdateText } from '../format.js';
import { _, fill, n_ } from '../i18n.js';
import { isRefreshing } from '../state.js';

const SECOND = 1000;
const MINUTE_SECS = 60;
const HOUR = 60 * 60 * SECOND;

function line(text, kind = 'plain') {
    return { text, kind };
}

function liveAccounts(state) {
    return state.accounts.filter(account => !account.hidden && account.refresh?.mode === 'live');
}

function providerNames(accounts) {
    return [...new Set(accounts.map(account => account.providerName))];
}

function liveSeconds(accounts) {
    const intervals = accounts.map(account => account.refresh.intervalSecs).filter(secs => secs !== null);
    return intervals.length > 0 ? Math.min(...intervals) : MINUTE_SECS;
}

function liveText(seconds, count) {
    if (seconds === MINUTE_SECS)
        return n_(
            'Live — every minute while {providers} is active',
            'Live — every minute while {providers} are active',
            count
        );
    return n_(
        'Live — every {interval} while {providers} is active',
        'Live — every {interval} while {providers} are active',
        count
    );
}

function offlineLines(state, now) {
    const first = state.lastSuccessAt
        ? line(fill(_('Outdated · updated {ago}'), { ago: agoText(state.lastSuccessAt, now) }), 'notice')
        : line(_('Offline'), 'notice');
    const retry = state.nextRefreshAt && state.nextRefreshAt > now;
    const second = retry
        ? line(fill(_('Offline — retrying in {duration}'), { duration: duration(state.nextRefreshAt - now, true) }))
        : line(_('Offline'));
    return { first, second, tip: null };
}

function liveLines(state, accounts, now) {
    const names = providerNames(accounts);
    const seconds = liveSeconds(accounts);
    const values = { providers: names.join(', '), interval: duration(seconds * SECOND) };
    const first = state.lastSuccessAt
        ? line(fill(_('Updated {ago}'), { ago: agoText(state.lastSuccessAt, now) }))
        : null;
    const live = liveText(seconds, names.length);
    const tip = n_(
        '{providers} is writing new usage logs, so it is checked every {interval}. Back to the normal interval after 10 minutes without activity.',
        '{providers} are writing new usage logs, so they are checked every {interval}. Back to the normal interval after 10 minutes without activity.',
        names.length
    );
    return { first, second: line(fill(live, values), 'live'), tip: fill(tip, values) };
}

function idleText(state, now, hour12) {
    const last = state.lastSuccessAt ? clockTime(state.lastSuccessAt, hour12) : null;
    const next = state.nextRefreshAt && state.nextRefreshAt > now ? clockTime(state.nextRefreshAt, hour12) : null;
    if (last && next) return fill(_('Updated {time} · next at {next}'), { time: last, next });
    if (state.nextRefreshAt) return nextUpdateText(state.nextRefreshAt, now);
    if (last) return fill(_('Updated {time}'), { time: last });
    return '';
}

function stateLines(state, now, hour12) {
    if (state.offline) return offlineLines(state, now);
    if (isRefreshing(state)) return { first: null, second: line(_('Updating…')), tip: null };
    const live = liveAccounts(state);
    if (live.length > 0) return liveLines(state, live, now);
    return { first: null, second: line(idleText(state, now, hour12)), tip: null };
}

export function footerLines(view, now, hour12) {
    if (view.kind === 'unavailable') return { first: null, second: line(_('Service not running')), tip: null };
    if (!view.state) return { first: null, second: line(view.kind === 'loading' ? _('Connecting…') : ''), tip: null };
    return stateLines(view.state, now, hour12);
}

export function footerNeedsSeconds(view, now) {
    const state = view.state;
    if (!state?.offline || !state.nextRefreshAt) return false;
    const left = state.nextRefreshAt - now;
    return left > 0 && left < HOUR;
}
