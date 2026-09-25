.pragma library

.import "Format.js" as Format
.import "PanelLayout.js" as PanelLayout
.import "Providers.js" as Providers
.import "Quota.js" as Quota
.import "State.js" as State

function windowTitle(lang, item) {
    return Format.windowLabel(lang, {
        id: item.windowId,
        label: item.windowLabel ?? item.windowId ?? ""
    });
}

function findWindow(account, windowId) {
    return account?.windows.find(window => window.id === windowId) ?? null;
}

function accountTitle(account, accounts) {
    return Providers.accountTitle(account, State.showsName(account, accounts));
}

function itemTitle(lang, item, account, accounts) {
    const window = windowTitle(lang, item);
    if (account !== null)
        return `${accountTitle(account, accounts)} · ${window}`;
    if (PanelLayout.showsCount(item))
        return `${item.providerName} ${Format.panelCount(item.accountCount)} · ${window}`;
    return `${item.providerName} · ${window}`;
}

function itemDetail(lang, window, now, display) {
    if (window === null)
        return "";
    const note = Quota.paceNote(lang, window, now, false);
    const reset = Quota.trailingText(lang, window, now, display.resetFormat, false, display.timeFormat);
    return note === null ? reset : `${note.text} · ${reset}`;
}

function pinnedRow(lang, state, item, now, accounts) {
    const account = accounts.find(candidate => candidate.id === item.accountId) ?? null;
    return {
        key: PanelLayout.itemKey(item),
        provider: item.logo,
        title: itemTitle(lang, item, account, accounts),
        reading: Format.readingFor(lang, item.valuePercent, state.display.valueMode),
        tone: item.tone,
        detail: itemDetail(lang, findWindow(account, item.windowId), now, state.display)
    };
}

function windowRow(lang, state, account, window, accounts) {
    return {
        key: `${account.id}\n${window.id}`,
        provider: account.provider,
        title: `${accountTitle(account, accounts)} · ${Format.windowLabel(lang, window)}`,
        reading: Format.readingFor(lang, Quota.shownPercent(window, state.display.valueMode), state.display.valueMode),
        tone: window.tone,
        detail: ""
    };
}

function restRows(lang, state, pinnedKeys, accounts) {
    const rows = [];
    for (const account of accounts.filter(State.hasQuotas))
        for (const window of State.shownWindows(account).filter(Quota.hasData))
            rows.push(windowRow(lang, state, account, window, accounts));
    return rows.filter(row => !pinnedKeys.includes(row.key));
}

function rows(lang, state, now) {
    if (state === null)
        return {
            pinned: [],
            rest: []
        };
    const accounts = State.visibleAccounts(state);
    const pinned = state.panelItems.map(item => pinnedRow(lang, state, item, now, accounts));
    return {
        pinned,
        rest: restRows(lang, state, pinned.map(row => row.key), accounts)
    };
}

function isEmpty(tipRows) {
    return tipRows.pinned.length === 0 && tipRows.rest.length === 0;
}
