.pragma library

.import "Account.js" as Account
.import "I18n.js" as I18n

const STATUSES = {
    signed_out: ["error", I18n.N("Signed out")],
    error: ["error", I18n.N("Couldn't refresh")],
    no_subscription: ["warning", I18n.N("No active subscription")],
    stale: ["warning", I18n.N("Outdated")]
};

function groups(accounts) {
    const found = [];
    for (const account of accounts) {
        const group = found.find(entry => entry.provider === account.provider);
        if (group)
            group.accounts.push(account);
        else
            found.push({
                provider: account.provider,
                providerName: account.providerName,
                accounts: [account]
            });
    }
    return found;
}

function rows(accounts) {
    const found = [];
    for (const group of groups(accounts)) {
        found.push({
            kind: "provider",
            key: `provider:${group.provider}`,
            provider: group.provider,
            title: group.providerName,
            account: null
        });
        for (const account of group.accounts)
            found.push({
                kind: "account",
                key: account.id,
                provider: account.provider,
                title: "",
                account
            });
    }
    return found;
}

function subtitle(lang, account) {
    const parts = [account.label ? account.email : null, account.plan];
    if (account.owner === "headroom")
        parts.push(I18n.tr(lang, "added in Headroom"));
    return parts.filter(Boolean).join(" · ");
}

function status(lang, account) {
    const known = STATUSES[Account.settledStatus(account)];
    if (!known)
        return {
            kind: "",
            text: ""
        };
    return {
        kind: known[0],
        text: I18n.tr(lang, known[1])
    };
}

function selected(accounts, id) {
    return accounts.find(account => account.id === id) ?? accounts[0] ?? null;
}

function signInTarget(account) {
    const recovery = account.recovery;
    if (recovery?.action === "sign_in")
        return recovery.accountId ?? account.id;
    if (recovery?.action === "cli_login")
        return recovery.accountId ?? null;
    if (account.owner === "headroom" && Account.isSignedOut(account))
        return account.id;
    return null;
}

function position(accounts, id) {
    return accounts.findIndex(account => account.id === id);
}
