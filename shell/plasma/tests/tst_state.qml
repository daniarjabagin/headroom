import QtQuick
import QtTest
import "../package/contents/ui/logic/Account.js" as Account
import "../package/contents/ui/logic/State.js" as State
import "../package/contents/ui/logic/Summary.js" as Summary

TestCase {
    readonly property date now: new Date("2026-09-23T10:00:00Z")

    function read(relative) {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl(relative), false);
        request.send();
        return request.responseText;
    }

    function sample() {
        return State.parseState(read("../dev/sample-state.json"));
    }

    function daemonJson() {
        return read("../../../crates/headroom-daemon/src/state/snapshots/state_full.json");
    }

    function keyMismatches(sample, daemon, path) {
        if (Array.isArray(sample) && Array.isArray(daemon))
            return sample.length === 0 || daemon.length === 0 ? [] : keyMismatches(sample[0], daemon[0], `${path}[0]`);
        const isObject = value => value !== null && typeof value === "object" && !Array.isArray(value);
        if (!isObject(sample) || !isObject(daemon))
            return [];
        const sampleKeys = Object.keys(sample).sort().join(",");
        const daemonKeys = Object.keys(daemon).sort().join(",");
        if (sampleKeys !== daemonKeys)
            return [`${path}: ${sampleKeys} vs ${daemonKeys}`];
        return Object.keys(daemon).reduce((found, key) => found.concat(keyMismatches(sample[key], daemon[key], `${path}.${key}`)), []);
    }

    function test_sample_accounts() {
        const state = sample();
        compare(state.accounts.length, 4);
        compare(state.headline, {
            accountId: "codex:1a2b3c4d5e6f",
            windowId: "session",
            remainingPercent: 62,
            tone: "good"
        });
        compare(state.nextRefreshAt.toISOString(), "2026-09-23T10:03:10.000Z");
        compare(state.offline, false);
        const personal = state.accounts[1];
        compare(personal.status, "stale");
        compare(personal.windows[1].pace.severity, "running_out");
        compare(personal.windows[1].pace.sparePercent, null);
        compare(personal.windows[0].pace.sparePercent, 4);
        compare(personal.windows[1].pace.runsOutAt.toISOString(), "2026-09-24T19:00:00.000Z");
        compare(personal.usage.provider, "codex");
        compare(state.accounts[0].balances[0].usdMicros, 12500000);
        compare(state.accounts[2].balances[0].unit, "requests");
        compare(state.accounts[3].status, "signed_out");
    }

    function test_sample_spend() {
        const state = sample();
        compare(state.accounts[2].usage.last30Days.unpricedModels, ["claude-next"]);
        compare(state.spend.today.costMicros, 18420000);
        compare(state.spend.today.providers.map(spend => spend.provider), ["codex", "claude"]);
        compare(state.spend.last30Days.partial, true);
    }

    function test_daemon_snapshot() {
        const json = daemonJson();
        const state = State.parseState(json);
        compare(state.spend.today.costMicros, 12400);
        compare(state.spend.yesterday.partial, true);
        compare(state.accounts[0].windows[1].pace.sparePercent, 47.5);
        compare(state.accounts[1].error.kind, "sign_in_expired");
        compare(state.accounts[2].hidden, true);
        compare(keyMismatches(JSON.parse(read("../dev/sample-state.json")), JSON.parse(json), "$"), []);
    }

    function test_edge_states() {
        for (const json of ["{", "{\"version\": 2}", "[]"]) {
            try {
                State.parseState(json);
                fail(`expected a state error for ${json}`);
            } catch (error) {
                verify(State.isStateError(error));
            }
        }
        const bare = State.parseState("{\"version\": 1}");
        compare(bare.accounts, []);
        compare(bare.spend, null);
        compare(bare.headline, null);
    }

    function test_headline() {
        const state = sample();
        compare(State.headlineWindow(state).label, "Session");
        compare(State.isHeadlineStale(state), false);
        compare(Summary.tooltip({
            kind: "ready",
            state
        }, now), {
            main: "Codex: work · Session",
            sub: "62% left · Resets in 2h 41m"
        });
        compare(Summary.tooltip({
            kind: "unavailable"
        }, now).sub, "Service not running");
    }

    function test_footer_line() {
        const state = sample();
        compare(Summary.footerLine({
            kind: "ready",
            state
        }, now).text, "Next update in 3m");
        state.offline = true;
        verify(Summary.footerLine({
            kind: "ready",
            state
        }, now).notice);
        state.accounts[0].status = "refreshing";
        state.offline = false;
        compare(Summary.footerLine({
            kind: "ready",
            state
        }, now).busy, true);
    }

    function test_account_notices() {
        const state = sample();
        compare(Account.statusSlot(state.accounts[1], false), "outdated");
        compare(Account.statusSlot(state.accounts[2], false), "warning");
        compare(Account.notices(state.accounts[2], false)[0].title, "Couldn't refresh Claude Code");
        compare(Account.notices(state.accounts[1], false)[0].kind, "warning");
        const signedOut = Account.notices(state.accounts[3], false);
        compare(signedOut.length, 1);
        compare(signedOut[0].actions.map(action => action.kind), ["copy", "retry"]);
        compare(signedOut[0].actions[0].value, "claude");
        const offline = Object.assign({}, state.accounts[2], {
            error: {
                kind: "network",
                message: "offline"
            }
        });
        compare(Account.statusSlot(offline, true), "outdated");
        compare(Account.notices(offline, true), []);
    }
}
