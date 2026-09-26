import { setLanguage } from '../src/i18n.js';
import { attentionText, collapsedAttention } from '../src/popup/collapsedAttention.js';
import { check } from './check.js';

function window(tone, hidden = false) {
    return { id: `w-${tone}`, tone, hidden };
}

function account(status, windows = [], error = null) {
    return { id: 'a', status, error, windows };
}

function single(status, windows = [], error = null) {
    return { kind: 'account', account: account(status, windows, error) };
}

function combined(members, windows) {
    return { kind: 'combined', members, group: { windows } };
}

const HTTP = { kind: 'http', message: 'HTTP 500' };
const NETWORK = { kind: 'network', message: 'offline' };
const SIGN_IN = { kind: 'not_signed_in', message: 'Sign in' };

function testTones() {
    check('empty is calm', collapsedAttention([], false), { kind: 'none', tone: null, count: 0 });
    check(
        'good and neutral stay calm',
        collapsedAttention([single('fresh', [window('good'), window('neutral')])], false).kind,
        'none'
    );
    check('warning tone', collapsedAttention([single('fresh', [window('warning')]), single('fresh')], false), {
        kind: 'tone',
        tone: 'warning',
        count: 1,
    });
    check(
        'worst tone wins',
        collapsedAttention([single('fresh', [window('warning')]), single('fresh', [window('critical')])], false),
        { kind: 'tone', tone: 'critical', count: 2 }
    );
    check(
        'hidden windows are ignored',
        collapsedAttention([single('fresh', [window('critical', true), window('good')])], false).kind,
        'none'
    );
}

function testNotices() {
    check('signed out is amber', collapsedAttention([single('signed_out')], false), {
        kind: 'notice',
        tone: 'warning',
        count: 1,
    });
    check('no subscription is amber', collapsedAttention([single('no_subscription')], false).tone, 'warning');
    check(
        'error is red and counts tone cards too',
        collapsedAttention(
            [single('signed_out'), single('error', [], HTTP), single('fresh', [window('warning')])],
            false
        ),
        { kind: 'notice', tone: 'critical', count: 3 }
    );
    check('refreshing with error is red', collapsedAttention([single('refreshing', [], HTTP)], false).tone, 'critical');
    check(
        'refreshing sign-in error is amber',
        collapsedAttention([single('refreshing', [], SIGN_IN)], false).tone,
        'warning'
    );
    check(
        'offline network failure has no notice',
        collapsedAttention([single('error', [], NETWORK)], true).kind,
        'none'
    );
    check('network failure online is red', collapsedAttention([single('error', [], NETWORK)], false).tone, 'critical');
}

function testCombined() {
    const calm = account('fresh', [window('critical')]);
    check('group uses its own windows', collapsedAttention([combined([calm], [window('good')])], false).kind, 'none');
    check('group window tone', collapsedAttention([combined([calm], [window('warning')])], false), {
        kind: 'tone',
        tone: 'warning',
        count: 1,
    });
    check(
        'group member status is one card',
        collapsedAttention([combined([calm, account('signed_out'), account('error', [], HTTP)], [])], false),
        { kind: 'notice', tone: 'critical', count: 1 }
    );
}

function testTexts() {
    check('calm has no text', attentionText({ kind: 'none', tone: null, count: 0 }), null);
    check('singular', attentionText({ kind: 'tone', tone: 'warning', count: 1 }), '1 needs attention');
    check('plural', attentionText({ kind: 'notice', tone: 'critical', count: 3 }), '3 need attention');
    setLanguage('ru');
    check('russian', attentionText({ kind: 'notice', tone: 'warning', count: 2 }), 'Требуют внимания: 2');
    check('russian one', attentionText({ kind: 'tone', tone: 'warning', count: 1 }), 'Требуют внимания: 1');
    setLanguage('en');
}

export function testCollapsedAttention() {
    testTones();
    testNotices();
    testCombined();
    testTexts();
}
