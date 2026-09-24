.pragma library

.import "Format.js" as Format
.import "I18n.js" as I18n
.import "Providers.js" as Providers
.import "State.js" as State

function line(text, notice, busy) {
    return {
        text,
        notice,
        busy
    };
}

function offlineLine(lang, state) {
    if (state.lastSuccessAt === null)
        return line(I18n.tr(lang, "Offline"), true, false);
    return line(I18n.tr(lang, "Offline — last update {time}", {
        time: Format.clockTime(state.lastSuccessAt)
    }), true, false);
}

function footerLine(lang, view, now) {
    if (view.kind === "unavailable")
        return line(I18n.tr(lang, "Service not running"), false, false);
    const state = view.state;
    if (!state)
        return line(view.kind === "loading" ? I18n.tr(lang, "Connecting…") : "", false, false);
    if (state.offline)
        return offlineLine(lang, state);
    if (State.isRefreshing(state))
        return line(I18n.tr(lang, "Updating…"), false, true);
    if (state.nextRefreshAt)
        return line(Format.nextUpdateText(lang, state.nextRefreshAt, now), false, false);
    if (state.lastSuccessAt)
        return line(I18n.tr(lang, "Updated {time}", {
            time: Format.clockTime(state.lastSuccessAt)
        }), false, false);
    return line("", false, false);
}

function headlineTitle(lang, state) {
    const headline = state.headline;
    const account = State.headlineAccount(state);
    const window = State.headlineWindow(state);
    const windowLabel = window === null ? headline.windowLabel ?? headline.windowId ?? "" : Format.windowLabel(lang, window);
    if (account === null && headline.combined && headline.accountCount !== null)
        return `${headline.providerName} ${Format.panelCount(headline.accountCount)} · ${windowLabel}`;
    if (account === null)
        return headline.accountLabel ? `${headline.accountLabel} · ${windowLabel}` : "Headroom";
    const visible = State.visibleAccounts(state);
    return `${Providers.accountTitle(account, State.showsName(account, visible))} · ${windowLabel}`;
}

function headlineDetail(lang, state, now) {
    const display = state.display;
    const reading = Format.readingFor(lang, State.headlinePercent(state.headline, display.valueMode), display.valueMode);
    const window = State.headlineWindow(state);
    if (window === null)
        return reading;
    return `${reading} · ${Format.resetText(lang, window.resetsAt, now, display.resetFormat, false)}`;
}

function tooltip(lang, view, now) {
    if (view.kind === "unavailable")
        return {
            main: "Headroom",
            sub: I18n.tr(lang, "Service not running")
        };
    if (view.kind === "error")
        return {
            main: "Headroom",
            sub: view.error
        };
    if (view.kind !== "ready" || view.state.headline === null)
        return {
            main: "Headroom",
            sub: view.kind === "ready" ? I18n.tr(lang, "No usage limits to show") : I18n.tr(lang, "Connecting…")
        };
    return {
        main: headlineTitle(lang, view.state),
        sub: headlineDetail(lang, view.state, now)
    };
}
