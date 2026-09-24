import { isRetrying, isSignedOut, lacksSubscription, subscriptionNote } from '../src/accountStatus.js';
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

export function testStatus() {
    testNoSubscription();
    testSignedOut();
    testSubscriptionNote();
    testRefreshing();
}
