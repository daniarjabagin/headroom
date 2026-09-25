export function visibleAccounts(state) {
    return state.accounts.filter(account => !account.hidden);
}

export function shownSpend(state) {
    return state.display.showSpend ? state.spend : null;
}

export function isEmptyState(state) {
    return visibleAccounts(state).length === 0 && !shownSpend(state);
}

export function needsToolsHint(state) {
    return visibleAccounts(state).length === 0 && Boolean(shownSpend(state));
}
