import QtQuick
import QtTest
import "../package/contents/ui/logic/PanelTip.js" as PanelTip
import "../package/contents/ui/logic/State.js" as State

TestCase {
    name: "PanelTip"

    function read(relative) {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl(relative), false);
        request.send();
        return request.responseText;
    }

    function sample(patch) {
        const raw = JSON.parse(read("../dev/sample-state.json"));
        return State.parseState(JSON.stringify(Object.assign(raw, patch ?? {})));
    }

    function now() {
        return new Date("2026-09-25T12:00:00Z");
    }

    function test_no_state_has_no_rows() {
        verify(PanelTip.isEmpty(PanelTip.rows("en", null, now())));
    }

    function test_panel_items_come_first_with_detail() {
        const rows = PanelTip.rows("en", sample(), now());
        compare(rows.pinned.length, 1);
        const pinned = rows.pinned[0];
        compare(pinned.key, "codex:1a2b3c4d5e6f\nsession");
        compare(pinned.provider, "codex");
        compare(pinned.title, "Codex: work · Session");
        compare(pinned.reading, "62% left");
        compare(pinned.tone, "good");
        verify(pinned.detail !== "");
    }

    function test_rest_lists_every_other_visible_window_once() {
        const rows = PanelTip.rows("en", sample(), now());
        const keys = rows.rest.map(row => row.key);
        verify(!keys.includes("codex:1a2b3c4d5e6f\nsession"));
        verify(keys.includes("codex:1a2b3c4d5e6f\nweekly"));
        verify(keys.includes("codex:9f8e7d6c5b4a\nweekly"));
        verify(keys.includes("zai:7b8c9d0e1f2a\ntools"));
        compare(keys.length, 9);
        compare(rows.rest.every(row => row.detail === ""), true);
        const critical = rows.rest.find(row => row.key === "codex:9f8e7d6c5b4a\nweekly");
        compare(critical.reading, "17% left");
        compare(critical.tone, "critical");
    }

    function test_hidden_accounts_are_left_out() {
        const raw = JSON.parse(read("../dev/sample-state.json"));
        raw.accounts.find(account => account.id === "zai:7b8c9d0e1f2a").hidden = true;
        const rows = PanelTip.rows("en", State.parseState(JSON.stringify(raw)), now());
        verify(!rows.rest.some(row => row.provider === "zai"));
    }

    function test_icon_mode_lists_everything_below_no_pinned_rows() {
        const rows = PanelTip.rows("en", sample({
            panel_items: []
        }), now());
        compare(rows.pinned.length, 0);
        compare(rows.rest.length, 10);
    }

    function test_combined_item_without_account_uses_its_count() {
        const rows = PanelTip.rows("en", sample({
            panel_items: [
                {
                    account_id: "combined:codex",
                    provider: "codex",
                    provider_name: "Codex",
                    window: "weekly",
                    window_label: "Weekly",
                    remaining_percent: 50,
                    used_percent: 50,
                    value_percent: 50,
                    tone: "warning",
                    combined: true,
                    account_count: 2
                }
            ]
        }), now());
        compare(rows.pinned[0].title, "Codex ×2 · Weekly");
        compare(rows.pinned[0].detail, "");
    }
}
