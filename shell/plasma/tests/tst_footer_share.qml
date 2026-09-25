import QtQuick
import QtTest
import "../package/contents/ui/logic/Combined.js" as Combined
import "../package/contents/ui/logic/FooterStatus.js" as FooterStatus
import "../package/contents/ui/logic/Settings.js" as Settings
import "../package/contents/ui/logic/Share.js" as Share
import "../package/contents/ui/logic/State.js" as State

TestCase {
    name: "FooterShare"

    readonly property var now: new Date(2026, 8, 25, 12, 25, 30)

    function read(relative) {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl(relative), false);
        request.send();
        return request.responseText;
    }

    function sample() {
        return State.parseState(read("../dev/sample-state.json"));
    }

    function ready(overrides, display) {
        const state = sample();
        const merged = Object.assign({}, state, {
            offline: false,
            lastSuccessAt: new Date(now.getTime() - 90000),
            nextRefreshAt: new Date(now.getTime() + 4 * 60000),
            accounts: state.accounts.map(account => Object.assign({}, account, {
                    status: account.status === "refreshing" ? "fresh" : account.status,
                    refresh: null
                }))
        }, overrides ?? {});
        merged.display = Object.assign({}, state.display, display ?? {});
        return {
            kind: "ready",
            state: merged
        };
    }

    function test_idle_footer() {
        const status = FooterStatus.footerStatus("en", ready(), now, "Headroom 0.6.0");
        compare(status.first.text, "Updated 1m ago");
        compare(status.second.text, "Next update in 4m");
        verify(!status.live && !status.stale && !status.busy);
    }

    function test_idle_footer_with_clock_times() {
        const view = ready({}, {
            resetFormat: "exact",
            timeFormat: "12h"
        });
        view.state.lastSuccessAt = new Date(2026, 8, 25, 12, 25);
        view.state.nextRefreshAt = new Date(2026, 8, 25, 12, 30);
        const status = FooterStatus.footerStatus("en", view, now, "");
        compare(status.first.text, "Updated 12:25 PM");
        compare(status.second.text, "Next at 12:30 PM");
        compare(FooterStatus.footerStatus("en", ready({
            lastSuccessAt: new Date(2026, 8, 25, 12, 25),
            nextRefreshAt: new Date(2026, 8, 25, 12, 30)
        }, {
            resetFormat: "exact",
            timeFormat: "24h"
        }), now, "").second.text, "Next at 12:30");
    }

    function test_live_footer_needs_activity() {
        const view = ready();
        view.state.accounts[0].refresh = {
            mode: "live",
            intervalSecs: 60,
            nextAt: null,
            reason: "activity"
        };
        view.state.accounts[2].refresh = {
            mode: "live",
            intervalSecs: 60,
            nextAt: null,
            reason: "backoff"
        };
        const status = FooterStatus.footerStatus("en", view, now, "");
        verify(status.live);
        compare(status.second.text, "Live — every 1m · Codex");
        verify(status.tip.startsWith("Codex wrote new usage"));
        view.state.accounts[0].refresh.reason = "backoff";
        verify(!FooterStatus.footerStatus("en", view, now, "").live);
    }

    function test_stale_footer_when_offline() {
        const status = FooterStatus.footerStatus("en", ready({
            offline: true,
            lastSuccessAt: new Date(now.getTime() - 12 * 60000),
            nextRefreshAt: new Date(now.getTime() + 45000)
        }), now, "");
        verify(status.stale);
        compare(status.first, {
            text: "Outdated · updated 12m ago",
            notice: true
        });
        compare(status.second.text, "Offline — retrying in 45s");
        compare(FooterStatus.footerStatus("ru", ready({
            offline: true,
            lastSuccessAt: null,
            nextRefreshAt: null
        }), now, "").second.text, "Нет сети");
    }

    function test_refreshing_footer_is_busy() {
        const view = ready();
        view.state.accounts[0].status = "refreshing";
        const status = FooterStatus.footerStatus("en", view, now, "");
        verify(status.busy);
        compare(status.second.text, "Updating…");
    }

    function test_footer_before_state_keeps_version_line() {
        const status = FooterStatus.footerStatus("en", {
            kind: "unavailable",
            state: null
        }, now, "Headroom 0.6.0");
        compare(status.first.text, "Headroom 0.6.0");
        compare(status.second.text, "Service not running");
    }

    function test_file_and_folder_names() {
        compare(Share.fileName("codex", new Date(2026, 8, 5, 9, 7)), "headroom-codex-2026-09-05-0907.png");
        compare(Share.folderPath("file:///home/ada/Pictures"), "/home/ada/Pictures/Headroom");
        compare(Share.folderPath("file:///home/ada/My%20Pictures/"), "/home/ada/My Pictures/Headroom");
        compare(Share.folderPath(""), "");
        compare(Share.folderUrl("/home/ada/My Pictures/Headroom"), "file:///home/ada/My%20Pictures/Headroom");
        compare(Share.folderCommand("/home/ada/it's"), "mkdir -p '/home/ada/it'\\''s'");
    }

    function test_share_model_has_no_personal_data() {
        const state = sample();
        const card = Combined.cards(state, State.visibleAccounts(state))[0];
        const shared = Share.model("en", card, "Pro", state.headline, now, state.display);
        compare(shared.provider, "codex");
        compare(shared.title, "CODEX LIMITS · 25 SEP 2026");
        compare(shared.hero.window.id, state.headline.windowId);
        compare(shared.hero.value, "62%");
        compare(shared.hero.suffix, "left");
        verify(shared.hero.sub.startsWith("Session · "));
        const text = Share.copyText(shared);
        verify(text.startsWith("Codex · Pro\nSession: 62% left · "));
        verify(!text.includes("work"));
        verify(!text.includes("@"));
    }

    function test_hero_picks_lowest_window_when_headline_elsewhere() {
        const state = sample();
        const cards = Combined.cards(state, State.visibleAccounts(state));
        const card = cards.find(candidate => candidate.account.provider === "claude" && State.shownWindows(candidate.account).length > 1);
        const hero = Share.heroWindow(card, state.headline);
        const lowest = State.shownWindows(card.account).filter(window => window.remainingPercent !== null).sort((first, second) => first.remainingPercent - second.remainingPercent)[0];
        compare(hero.id, lowest.id);
    }

    function test_share_uses_used_mode_and_palette() {
        const state = sample();
        const card = Combined.cards(state, State.visibleAccounts(state))[0];
        const display = Object.assign({}, Settings.parseDisplay(null), {
            valueMode: "used"
        });
        const shared = Share.model("ru", card, "", state.headline, now, display);
        compare(shared.hero.value, "38%");
        compare(shared.hero.suffix, "использовано");
        compare(Share.copyText(shared).split("\n")[0], "Codex");
        compare(Share.fractions(shared.hero.window, "used"), [0.38]);
        compare(Share.palette(true).background, "#151617");
        compare(Share.palette(false).background, "#F0F0ED");
        compare(Share.layout().width * Share.layout().scale, 1200);
        compare(Share.layout().height * Share.layout().scale, 630);
    }
}
