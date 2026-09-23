const NO_SUBSCRIPTION = 'no_subscription';
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

export function subscriptionNote(error) {
    const message = error?.message ?? null;
    if (message === null || message === error.kind) return null;
    const text = normalized(message);
    if (text === '' || text === NO_SUBSCRIPTION_TITLE || text === normalized(NO_SUBSCRIPTION)) return null;
    return message;
}
