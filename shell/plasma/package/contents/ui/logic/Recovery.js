.pragma library

.import "Commands.js" as Commands
.import "I18n.js" as I18n
.import "Registry.js" as Registry

const WAIT_ERRORS = ["unsupported", "no_provider", "rate_limited", "no_subscription"];

function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function text(value) {
    return typeof value === "string" && value.trim().length > 0 ? value.trim() : null;
}

function parseRecovery(raw) {
    if (!isObject(raw))
        return null;
    if (raw.action === "retry")
        return {
            action: "retry"
        };
    if (raw.action === "sign_in")
        return {
            action: "sign_in",
            accountId: text(raw.account_id)
        };
    const command = text(raw.command);
    if (raw.action === "cli_login" && command !== null)
        return {
            action: "cli_login",
            command
        };
    return null;
}

function fallback(errorKind, signedOut) {
    if (!signedOut && (errorKind === undefined || WAIT_ERRORS.includes(errorKind)))
        return null;
    return {
        action: "retry"
    };
}

function effective(account, signedOut) {
    return account.recovery ?? fallback(account.error?.kind, signedOut);
}

function retryAction(lang, account) {
    return {
        kind: "retry",
        label: I18n.tr(lang, "Retry"),
        value: account.id,
        busy: account.status === "refreshing"
    };
}

function signInAction(lang, account, providers, msgid) {
    const method = Registry.findProvider(providers, account.provider)?.method;
    return {
        kind: Commands.opensTerminal(method) ? "signin" : "settings",
        label: I18n.tr(lang, msgid),
        value: account.provider
    };
}

function copyAction(lang, command) {
    return {
        kind: "copy",
        label: I18n.tr(lang, "Copy command"),
        doneLabel: I18n.tr(lang, "Copied"),
        value: command
    };
}

function actions(lang, account, providers, signedOut) {
    const recovery = effective(account, signedOut);
    switch (recovery?.action) {
    case "sign_in":
        return [signInAction(lang, account, providers, I18n.N("Sign in again…"))];
    case "cli_login":
        return [copyAction(lang, recovery.command), retryAction(lang, account)];
    case "retry":
        return signedOut ? [signInAction(lang, account, providers, I18n.N("Sign in…")), retryAction(lang, account)] : [retryAction(lang, account)];
    default:
        return [];
    }
}

function terminalHint(lang, account) {
    const recovery = effective(account, false);
    if (recovery?.action !== "cli_login")
        return "";
    return I18n.tr(lang, "Run `{command}` in a terminal — Headroom picks it up automatically.", {
        command: recovery.command
    });
}

function signedOutDetail(lang, account) {
    const hint = terminalHint(lang, account);
    if (hint !== "")
        return hint;
    if (effective(account, true)?.action === "sign_in")
        return I18n.tr(lang, "Headroom's sign-in for this account has expired. Sign in again or remove the account.");
    return I18n.tr(lang, "Sign in again through Headroom (Settings → Accounts → Add Account) or remove the account.");
}

function buttonLabel(action, copiedValue) {
    if (action.kind === "copy" && action.value === copiedValue)
        return action.doneLabel;
    return action.label;
}
