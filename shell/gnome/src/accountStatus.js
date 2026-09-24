const NO_SUBSCRIPTION = 'no_subscription';
const SIGNED_OUT = 'signed_out';
const SIGN_IN_ERRORS = new Set(['not_signed_in', 'sign_in_expired']);
const NO_SUBSCRIPTION_TITLE = 'no active subscription';

function normalized(text) {
    return text
        .toLowerCase()
        .replace(/[^\p{L}\p{N}]+/gu, ' ')
        .trim();
}

export function lacksSubscription(account) {
    if (account.status === NO_SUBSCRIPTION) return true;
    return account.status === 'refreshing' && account.error?.kind === NO_SUBSCRIPTION;
}

export function isSignedOut(account) {
    if (account.status === SIGNED_OUT) return true;
    return account.status === 'refreshing' && SIGN_IN_ERRORS.has(account.error?.kind);
}

export function isRetrying(account) {
    return account.status === 'refreshing';
}

export function subscriptionNote(error) {
    const message = error?.message ?? null;
    if (message === null || message === error.kind) return null;
    const text = normalized(message);
    if (text === '' || text === NO_SUBSCRIPTION_TITLE || text === normalized(NO_SUBSCRIPTION)) return null;
    return message;
}
