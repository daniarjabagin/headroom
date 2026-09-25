import {
    isRetrying,
    isSignedOut,
    lacksSubscription,
    noticeShape,
    recoveryActions,
    RETRY_FEEDBACK_MS,
    retryBusy,
    subscriptionNote,
} from '../src/accountStatus.js';
import { isRefreshing, parseState } from '../src/state.js';
import { check } from './check.js';

function stateWith(accounts) {
    return parseState(JSON.stringify({ version: 1, accounts }));
}

function lapsed(status, message) {
    return {
        id: 'codex:7c6b5a4f3e2d',
        provider: 'codex',
        status,
        error: { kind: 'no_subscription', message },
        windows: [],
    };
}

function testNoSubscription() {
    const [account] = stateWith([lapsed('no_subscription', 'ChatGPT Plus ended on Sep 20')]).accounts;
    check('no subscription status', account.status, 'no_subscription');
    check('no subscription error', account.error.kind, 'no_subscription');
    check('lacks subscription', lacksSubscription(account), true);
    const [refreshing] = stateWith([lapsed('refreshing', 'x')]).accounts;
    check('still lapsed while refreshing', lacksSubscription(refreshing), true);
    const [failing] = stateWith([{ ...lapsed('error', 'x'), error: { kind: 'network', message: 'x' } }]).accounts;
    check('other errors keep limits', lacksSubscription(failing), false);
}

function signedOut(status, kind) {
    return { id: 'grok:6d5c4b3a2f1e', provider: 'grok', status, error: { kind, message: 'x' }, windows: [] };
}

function testSignedOut() {
    const accounts = stateWith([
        signedOut('signed_out', 'sign_in_expired'),
        signedOut('refreshing', 'sign_in_expired'),
        signedOut('refreshing', 'not_signed_in'),
        signedOut('refreshing', 'network'),
        signedOut('error', 'sign_in_expired'),
    ]).accounts;
    check(
        'signed out, also while retrying',
        accounts.map(account => isSignedOut(account)),
        [true, true, true, false, false]
    );
    check(
        'retrying only while refreshing',
        accounts.map(account => isRetrying(account)),
        [false, true, true, true, false]
    );
}

function testSubscriptionNote() {
    const note = message => subscriptionNote({ kind: 'no_subscription', message });
    check('detail kept', note('ChatGPT Plus ended on Sep 20'), 'ChatGPT Plus ended on Sep 20');
    check('title repeated', note('No active subscription.'), null);
    check('kind repeated', note('no_subscription'), null);
    check('no error', subscriptionNote(null), null);
}

function testRefreshing() {
    check('refreshing', isRefreshing(stateWith([lapsed('refreshing', 'x')])), true);
    check('idle', isRefreshing(stateWith([lapsed('no_subscription', 'x')])), false);
}

function failing(status, kind, recovery) {
    return {
        id: 'claude:0a1b2c3d4e5f',
        provider: 'claude',
        owner: 'cli',
        status,
        error: { kind, message: 'x' },
        ...(recovery === undefined ? {} : { recovery }),
        windows: [],
    };
}

function shapes(statuses, kind, recovery, offline = false) {
    const accounts = stateWith(statuses.map(status => failing(status, kind, recovery))).accounts;
    return accounts.map(account => JSON.stringify(noticeShape(account, offline)));
}

function testNoticeStaysWhileRefreshing() {
    const flip = ['error', 'refreshing', 'error'];
    const [error, refreshing, again] = shapes(flip, 'network', { action: 'retry' });
    check('error notice shown', JSON.parse(error), { notice: 'error', actions: ['retry'], command: null });
    check('error notice kept while refreshing', [refreshing, again], [error, error]);
    const signedOut = shapes(['signed_out', 'refreshing', 'signed_out'], 'sign_in_expired', {
        action: 'cli_login',
        command: 'claude auth login',
    });
    check('sign-in notice kept while refreshing', new Set(signedOut).size, 1);
    const legacy = shapes(flip, 'invalid_response', undefined);
    check('older daemon keeps the notice too', new Set(legacy).size, 1);
    check('offline network errors show no notice', shapes(flip, 'network', null, true), ['null', 'null', 'null']);
    check('healthy refresh shows no notice', noticeShape(stateWith([lapsed('fresh', 'x')]).accounts[0], false), null);
}

function actionsFor(status, kind, recovery) {
    return recoveryActions(stateWith([failing(status, kind, recovery)]).accounts[0]);
}

function testRecoveryActions() {
    check('retry', actionsFor('error', 'network', { action: 'retry' }), ['retry']);
    check(
        'sign in again',
        actionsFor('signed_out', 'sign_in_expired', { action: 'sign_in', account_id: 'claude:0a1b2c3d4e5f' }),
        ['sign_in', 'retry']
    );
    check(
        'copy command',
        actionsFor('signed_out', 'not_signed_in', { action: 'cli_login', command: 'claude auth login' }),
        ['copy_command', 'retry']
    );
    check('cli login on api key error', actionsFor('error', 'api_key_only', { action: 'cli_login', command: 'x' }), [
        'copy_command',
        'retry',
    ]);
    check('retry for a signed out cli account', actionsFor('signed_out', 'sign_in_expired', { action: 'retry' }), [
        'retry',
    ]);
    check('older daemon signed out', actionsFor('signed_out', 'sign_in_expired', undefined), ['sign_in', 'retry']);
    check('older daemon error', actionsFor('error', 'network', undefined), ['retry']);
    check('account changed retries', actionsFor('error', 'account_changed', { action: 'retry' }), ['retry']);
}

function testRetryBusy() {
    check('busy while refreshing', retryBusy(true, null, 0), true);
    check('idle without a click', retryBusy(false, null, 5000), false);
    check('busy right after a click', retryBusy(false, 1000, 1000 + RETRY_FEEDBACK_MS - 1), true);
    check('idle after the feedback', retryBusy(false, 1000, 1000 + RETRY_FEEDBACK_MS), false);
}

function recoveryOf(recovery) {
    return stateWith([failing('signed_out', 'sign_in_expired', recovery)]).accounts[0].recovery;
}

function testRecoveryParsing() {
    check('retry parsed', recoveryOf({ action: 'retry' }), { action: 'retry' });
    check('sign in parsed', recoveryOf({ action: 'sign_in', account_id: 'claude:0a1b2c3d4e5f' }), {
        action: 'sign_in',
        accountId: 'claude:0a1b2c3d4e5f',
    });
    check('cli login parsed', recoveryOf({ action: 'cli_login', command: 'claude auth login' }), {
        action: 'cli_login',
        command: 'claude auth login',
    });
    check('cli login without command', recoveryOf({ action: 'cli_login' }), null);
    check('unknown action', recoveryOf({ action: 'reboot' }), null);
    check('missing recovery', recoveryOf(undefined), null);
    check('null recovery', recoveryOf(null), null);
    const [changed] = stateWith([failing('error', 'account_changed', { action: 'retry' })]).accounts;
    check('account changed kind', changed.error.kind, 'account_changed');
}

export function testStatus() {
    testRecoveryParsing();
    testNoticeStaysWhileRefreshing();
    testRecoveryActions();
    testRetryBusy();
    testNoSubscription();
    testSignedOut();
    testSubscriptionNote();
    testRefreshing();
}
