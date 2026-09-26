const NO_SUBSCRIPTION = 'no_subscription';
const SIGNED_OUT = 'signed_out';
const SIGN_IN_ERRORS = new Set(['not_signed_in', 'sign_in_expired']);
const NO_SUBSCRIPTION_TITLE = 'no active subscription';

export const RETRY_FEEDBACK_MS = 800;

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

export function isQuietlyLimited(account) {
    return account.error?.kind === 'rate_limited' && account.updatedAt !== null;
}

function hasErrorNotice(account, offline) {
    if (account.error === null) return false;
    if (account.status !== 'error' && account.status !== 'refreshing') return false;
    if (isQuietlyLimited(account)) return false;
    return !(offline && account.error.kind === 'network');
}

export function accountNotice(account, offline) {
    if (isSignedOut(account)) return 'signed_out';
    if (lacksSubscription(account)) return 'no_subscription';
    return hasErrorNotice(account, offline) ? 'error' : null;
}

function cliLoginActions(recovery) {
    const copy = recovery.command ? ['copy_command'] : [];
    return recovery.accountId ? ['cli_sign_in', 'retry', ...copy] : [...copy, 'retry'];
}

export function recoveryActions(account) {
    const action = account.recovery?.action;
    if (action === 'sign_in') return ['sign_in', 'retry'];
    if (action === 'cli_login') return cliLoginActions(account.recovery);
    if (account.recovery === null && isSignedOut(account)) return ['sign_in', 'retry'];
    return ['retry'];
}

export function noticeShape(account, offline) {
    const notice = accountNotice(account, offline);
    if (notice === null) return null;
    return { notice, actions: recoveryActions(account), command: account.recovery?.command ?? null };
}

export function retryBusy(refreshing, clickedAt, now) {
    return refreshing || (clickedAt !== null && now - clickedAt < RETRY_FEEDBACK_MS);
}
