.pragma library

.import "I18n.js" as I18n
.import "Providers.js" as Providers

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

function signedOutNotice(lang, account) {
    const info = Providers.providerInfo(account.provider);
    const actions = [retryAction(lang, account)];
    if (info.signInCommand)
        actions.unshift({
            kind: "copy",
            label: I18n.tr(lang, "Copy command"),
            value: info.signInCommand
        });
    return {
        kind: "signin",
        title: I18n.tr(lang, "Signed out of {provider}", {
            provider: info.name
        }),
        detail: info.signInCommand ? I18n.tr(lang, "Run \"{command}\" and sign in, then Retry", {
            command: info.signInCommand
        }) : I18n.tr(lang, "Sign in again, then Retry"),
        actions
    };
}

function errorNotice(lang, account) {
    return {
        kind: "error",
        title: I18n.tr(lang, "Couldn't refresh {provider}", {
            provider: Providers.providerInfo(account.provider).name
        }),
        detail: account.error?.message ?? "",
        actions: [retryAction(lang, account)]
    };
}

function providerNotice(notice) {
    return {
        kind: notice.tone === "critical" ? "error" : "warning",
        title: notice.text,
        detail: "",
        actions: []
    };
}

function notices(lang, account, offline) {
    if (account.status === "signed_out")
        return [signedOutNotice(lang, account)];
    const rows = account.notices.map(providerNotice);
    if (account.status === "error" && !failedOffline(account, offline))
        rows.unshift(errorNotice(lang, account));
    return rows;
}

function showsQuotas(account) {
    return account.status !== "signed_out";
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

function spendRows(lang, account, display) {
    if (!showsSpend(account, display))
        return [];
    const provider = Providers.providerInfo(account.provider).name;
    return SPEND_PERIODS.map(([msgid, key]) => {
        const title = I18n.tr(lang, msgid);
        return {
            title,
            breakdownTitle: `${title} · ${provider}`,
            totals: account.usage[key]
        };
    });
}
