.pragma library

.import "FormatTime.js" as FormatTime
.import "I18n.js" as I18n
.import "State.js" as State
.import "Summary.js" as Summary

const MS_PER_SECOND = 1000;
const NAME_SEPARATOR = ", ";

function line(text, notice) {
    return {
        text,
        notice: notice === true
    };
}

function status(first, second, extra) {
    return Object.assign({
        first,
        second,
        busy: false,
        live: false,
        stale: false,
        tip: ""
    }, extra ?? {});
}

function liveAccounts(state) {
    return State.visibleAccounts(state).filter(account => account.refresh?.mode === "live" && account.refresh.reason === "activity");
}

function liveNames(accounts) {
    return accounts.map(account => account.providerName).filter((name, index, all) => all.indexOf(name) === index).join(NAME_SEPARATOR);
}

function updatedText(lang, state, now) {
    if (state.lastSuccessAt === null)
        return "";
    if (state.display.resetFormat === "exact")
        return I18n.tr(lang, "Updated {time}", {
            time: FormatTime.clockTime(state.lastSuccessAt, state.display.timeFormat)
        });
    return I18n.tr(lang, "Updated {ago}", {
        ago: FormatTime.agoText(lang, state.lastSuccessAt, now)
    });
}

function nextText(lang, state, now) {
    if (state.nextRefreshAt === null)
        return "";
    if (state.display.resetFormat === "exact")
        return I18n.tr(lang, "Next at {time}", {
            time: FormatTime.clockTime(state.nextRefreshAt, state.display.timeFormat)
        });
    return FormatTime.nextUpdateText(lang, state.nextRefreshAt, now);
}

function staleStatus(lang, state, now) {
    const first = state.lastSuccessAt === null ? I18n.tr(lang, "Outdated") : I18n.tr(lang, "Outdated · updated {ago}", {
        ago: FormatTime.agoText(lang, state.lastSuccessAt, now)
    });
    const second = state.nextRefreshAt !== null && state.nextRefreshAt > now ? I18n.tr(lang, "Offline — retrying in {duration}", {
        duration: FormatTime.duration(lang, state.nextRefreshAt - now, true)
    }) : I18n.tr(lang, "Offline");
    return status(line(first, true), line(second, true), {
        stale: true
    });
}

function liveStatus(lang, state, now, accounts) {
    const names = liveNames(accounts);
    const every = FormatTime.duration(lang, (accounts[0].refresh.intervalSecs ?? 60) * MS_PER_SECOND);
    return status(line(updatedText(lang, state, now)), line(I18n.tr(lang, "Live — every {duration} · {names}", {
        duration: every,
        names
    })), {
        live: true,
        tip: I18n.tr(lang, "{names} wrote new usage in the last 10 minutes, so it is checked every {duration}. Back to the usual interval after 10 minutes without activity.", {
            names,
            duration: every
        })
    });
}

function readyStatus(lang, state, now) {
    if (state.offline)
        return staleStatus(lang, state, now);
    if (State.isRefreshing(state))
        return status(line(updatedText(lang, state, now)), line(I18n.tr(lang, "Updating…")), {
            busy: true
        });
    const live = liveAccounts(state);
    if (live.length > 0)
        return liveStatus(lang, state, now, live);
    return status(line(updatedText(lang, state, now)), line(nextText(lang, state, now)));
}

function footerStatus(lang, view, now, versionText) {
    if (view.kind === "ready" && view.state)
        return readyStatus(lang, view.state, now);
    const summary = Summary.footerLine(lang, view, now);
    return status(line(versionText), line(summary.text, summary.notice), {
        busy: summary.busy
    });
}
