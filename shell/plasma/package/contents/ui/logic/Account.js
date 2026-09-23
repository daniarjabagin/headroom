.pragma library

.import "Providers.js" as Providers

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

function signedOutNotice(account) {
    const info = Providers.providerInfo(account.provider);
    const actions = [
        {
            kind: "retry",
            label: "Retry",
            value: account.id
        }
    ];
    if (info.signInCommand)
        actions.unshift({
            kind: "copy",
            label: "Copy command",
            value: info.signInCommand
        });
    return {
        kind: "signin",
        title: `Signed out of ${info.name}`,
        detail: info.signInCommand ? `Run "${info.signInCommand}" and sign in, then Retry` : "Sign in again, then Retry",
        actions
    };
}

function errorNotice(account) {
    return {
        kind: "error",
        title: `Couldn't refresh ${Providers.providerInfo(account.provider).name}`,
        detail: account.error?.message ?? "",
        actions: [
            {
                kind: "retry",
                label: "Retry",
                value: account.id
            }
        ]
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

function notices(account, offline) {
    if (account.status === "signed_out")
        return [signedOutNotice(account)];
    const rows = account.notices.map(providerNotice);
    if (account.status === "error" && !failedOffline(account, offline))
        rows.unshift(errorNotice(account));
    return rows;
}

function showsQuotas(account) {
    return account.status !== "signed_out";
}

function hasExtras(account) {
    return account.usage !== null || account.balances.length > 0;
}

function spendRows(usage) {
    if (usage === null)
        return [];
    return [["Today", usage.today], ["Yesterday", usage.yesterday], ["Last 30 Days", usage.last30Days]].map(([title, totals]) => ({
                title,
                totals
            }));
}
