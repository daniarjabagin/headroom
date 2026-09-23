.pragma library

.import "Format.js" as Format
.import "Providers.js" as Providers
.import "State.js" as State

function line(text, notice, busy) {
    return {
        text,
        notice,
        busy
    };
}

function footerLine(view, now) {
    if (view.kind === "unavailable")
        return line("Service not running", false, false);
    const state = view.state;
    if (!state)
        return line(view.kind === "loading" ? "Connecting…" : "", false, false);
    if (state.offline) {
        const since = state.lastSuccessAt ? ` — last update ${Format.clockTime(state.lastSuccessAt)}` : "";
        return line(`Offline${since}`, true, false);
    }
    if (State.isRefreshing(state))
        return line("Updating…", false, true);
    if (state.nextRefreshAt)
        return line(Format.nextUpdateText(state.nextRefreshAt, now), false, false);
    if (state.lastSuccessAt)
        return line(`Updated ${Format.clockTime(state.lastSuccessAt)}`, false, false);
    return line("", false, false);
}

function headlineTitle(state) {
    const account = State.headlineAccount(state);
    const window = State.headlineWindow(state);
    if (account === null || window === null)
        return "Headroom";
    const visible = State.visibleAccounts(state);
    return `${Providers.accountTitle(account, State.showsName(account, visible))} · ${window.label}`;
}

function headlineDetail(state, now) {
    const window = State.headlineWindow(state);
    const left = Format.percentLeft(state.headline.remainingPercent);
    if (window === null)
        return left;
    return `${left} · ${Format.resetText(window.resetsAt, now)}`;
}

function tooltip(view, now) {
    if (view.kind === "unavailable")
        return {
            main: "Headroom",
            sub: "Service not running"
        };
    if (view.kind === "error")
        return {
            main: "Headroom",
            sub: view.error
        };
    if (view.kind !== "ready" || view.state.headline === null)
        return {
            main: "Headroom",
            sub: view.kind === "ready" ? "No usage limits to show" : "Connecting…"
        };
    return {
        main: headlineTitle(view.state),
        sub: headlineDetail(view.state, now)
    };
}
