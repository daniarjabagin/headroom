.pragma library

.import "DaemonText.js" as DaemonText
.import "FormatTime.js" as FormatTime
.import "I18n.js" as I18n
.import "Recovery.js" as Recovery
.import "State.js" as State

const SIGNED_OUT_ERRORS = ["not_signed_in", "sign_in_expired"];
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

function isQuietlyLimited(account) {
    return account.error?.kind === "rate_limited" && (account.updatedAt ?? null) !== null;
}

function rateLimitNote(lang, account, timeFormat) {
    if (!isQuietlyLimited(account))
        return "";
    const next = account.refresh?.nextAt ?? null;
    if (next === null)
        return I18n.tr(lang, "Provider is limiting requests");
    return I18n.tr(lang, "Provider is limiting requests · next try {time}", {
        time: FormatTime.clockTime(next, timeFormat)
    });
}

function settledStatus(account) {
    if (account.status !== "refreshing" || account.error === null || isQuietlyLimited(account))
        return account.status;
    if (SIGNED_OUT_ERRORS.includes(account.error.kind))
        return "signed_out";
    if (account.error.kind === "no_subscription")
        return "no_subscription";
    return "error";
}

function isSignedOut(account) {
    return settledStatus(account) === "signed_out";
}

function signedOutNotice(lang, account, providers) {
    return {
        kind: "signin",
        title: I18n.tr(lang, "Signed out of {provider}", {
            provider: account.providerName
        }),
        detail: Recovery.signedOutDetail(lang, account),
        note: "",
        actions: Recovery.actions(lang, account, providers, true)
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
        actions: [Recovery.retryAction(lang, account)]
    };
}

function errorNotice(lang, account, providers) {
    return {
        kind: "error",
        title: I18n.tr(lang, "Couldn't refresh {provider}", {
            provider: account.providerName
        }),
        detail: account.error?.message ?? "",
        note: Recovery.terminalHint(lang, account),
        actions: Recovery.actions(lang, account, providers, false)
    };
}

function providerKind(tone) {
    if (tone === "critical")
        return "error";
    if (tone === "warning")
        return "warning";
    return "info";
}

function providerNotice(lang, notice) {
    return {
        kind: providerKind(notice.tone),
        title: DaemonText.notice(lang, notice.text),
        detail: "",
        note: "",
        actions: []
    };
}

function notices(lang, account, offline, providers) {
    if (isSignedOut(account))
        return [signedOutNotice(lang, account, providers)];
    const status = settledStatus(account);
    if (status === "no_subscription")
        return [noSubscriptionNotice(lang, account)];
    const rows = account.notices.map(notice => providerNotice(lang, notice));
    if (status === "error" && !failedOffline(account, offline))
        rows.unshift(errorNotice(lang, account, providers));
    return rows;
}

function plates(notices) {
    return notices.filter(notice => notice.kind !== "info");
}

function infoLines(notices) {
    return notices.filter(notice => notice.kind === "info");
}

function showsQuotas(account) {
    return State.hasQuotas(account) && !isSignedOut(account);
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

function removal(lang, account) {
    if (account.owner === "headroom")
        return {
            subtitle: I18n.tr(lang, "Deletes the sign-in Headroom created for this account"),
            confirmation: I18n.tr(lang, "Headroom deletes the sign-in it created for this account. The account itself is not affected.")
        };
    return {
        subtitle: I18n.tr(lang, "Stops showing this account; the {provider} CLI stays signed in", {
            provider: account.providerName
        }),
        confirmation: I18n.tr(lang, "Headroom will stop showing this account. The {provider} CLI stays signed in; you can sign in again through Headroom.", {
            provider: account.providerName
        })
    };
}
