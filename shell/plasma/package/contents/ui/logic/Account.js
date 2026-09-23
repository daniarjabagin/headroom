.pragma library

.import "I18n.js" as I18n
.import "Registry.js" as Registry
.import "State.js" as State

const PERMANENT_ERRORS = ["unsupported", "no_provider"];
const SPEND_PERIODS = [[I18n.N("Today"), "today"], [I18n.N("Yesterday"), "yesterday"], [I18n.N("Last 30 Days"), "last30Days"]];

function failedOffline(account, offline) {
    return offline && account.error?.kind === "network";
}

function statusSlot(account, offline) {
    if (account.status === "refreshing")
        return "refreshing";
    if (account.status === "stale" || (account.status === "error" && failedOffline(account, offline)))
        return "outdated";
    if (account.status === "error")
        return "warning";
    return "";
}

function retryAction(lang, account) {
    return {
        kind: "retry",
        label: I18n.tr(lang, "Retry"),
        value: account.id
    };
}

function signsInFromTerminal(providers, id) {
    return Registry.findProvider(providers, id)?.method.kind === "cli_login";
}

function signedOutActions(lang, account, providers) {
    const retry = retryAction(lang, account);
    if (!signsInFromTerminal(providers, account.provider))
        return [retry];
    return [{
            kind: "signin",
            label: I18n.tr(lang, "Sign in again…"),
            value: account.provider
        }, retry];
}

function signedOutNotice(lang, account, providers) {
    return {
        kind: "signin",
        title: I18n.tr(lang, "Signed out of {provider}", {
            provider: account.providerName
        }),
        detail: I18n.tr(lang, "Sign in again, then Retry"),
        note: "",
        actions: signedOutActions(lang, account, providers)
    };
}

function daemonNote(account) {
    const error = account.error;
    return error === null || error.message === error.kind ? "" : error.message;
}

function noSubscriptionNotice(lang, account) {
    return {
        kind: "warning",
        title: I18n.tr(lang, "No active subscription"),
        detail: I18n.tr(lang, "Limits aren't available for this account. Renew the plan or sign in with another account."),
        note: daemonNote(account),
        actions: [retryAction(lang, account)]
    };
}

function errorNotice(lang, account) {
    return {
        kind: "error",
        title: I18n.tr(lang, "Couldn't refresh {provider}", {
            provider: account.providerName
        }),
        detail: account.error?.message ?? "",
        note: "",
        actions: PERMANENT_ERRORS.includes(account.error?.kind) ? [] : [retryAction(lang, account)]
    };
}

function providerNotice(notice) {
    return {
        kind: notice.tone === "critical" ? "error" : "warning",
        title: notice.text,
        detail: "",
        note: "",
        actions: []
    };
}

function notices(lang, account, offline, providers) {
    if (account.status === "signed_out")
        return [signedOutNotice(lang, account, providers)];
    if (account.status === "no_subscription")
        return [noSubscriptionNotice(lang, account)];
    const rows = account.notices.map(providerNotice);
    if (account.status === "error" && !failedOffline(account, offline))
        rows.unshift(errorNotice(lang, account));
    return rows;
}

function showsQuotas(account) {
    return State.hasQuotas(account);
}

function showsSpend(account, display) {
    return account.usage !== null && display.showAccountSpend;
}

function showsTrend(account, display) {
    return showsQuotas(account) && account.usage !== null && display.showTrend;
}

function hasExtras(account, display) {
    return showsQuotas(account) && (showsSpend(account, display) || account.balances.length > 0);
}

function extrasAlwaysOpen(account) {
    return showsQuotas(account) && State.shownWindows(account).length === 0 && account.balances.length > 0;
}

function spendRows(lang, account, display) {
    if (!showsSpend(account, display))
        return [];
    const provider = account.providerName;
    return SPEND_PERIODS.map(([msgid, key]) => {
        const title = I18n.tr(lang, msgid);
        return {
            title,
            breakdownTitle: `${title} · ${provider}`,
            totals: account.usage[key]
        };
    });
}
