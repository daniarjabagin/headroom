pragma ComponentBehavior: Bound
import QtQuick
import QtTest
import "../package/contents/ui"
import "../package/contents/ui/logic/Settings.js" as Settings

TestCase {
    id: suite

    function item(accountId, value, tone) {
        return {
            accountId,
            windowId: "weekly",
            windowLabel: "Weekly",
            provider: "codex",
            logo: "codex",
            valuePercent: value,
            remainingPercent: value,
            usedPercent: 100 - value,
            evenPacePercent: 50,
            tone,
            combined: false,
            accountCount: 1
        };
    }

    function labels(compact) {
        const found = [];
        const walk = node => {
            if (node.objectName === "compactPercentLabel")
                found.push(node);
            for (const child of node.children)
                walk(child);
        };
        walk(compact);
        return found;
    }

    function make(properties) {
        const compact = createTemporaryObject(compactComponent, suite, properties);
        verify(compact !== null);
        return compact;
    }

    function test_several_mode_draws_one_item_per_panel_item() {
        const compact = make({
            items: [item("a", 72, "good"), item("b", 40, "warning")],
            display: Settings.parseDisplay({
                panel_mode: "several",
                panel_indicator: "none"
            })
        });
        compare(labels(compact).map(label => label.text), ["72%", "40%"]);
        verify(!compact.markShown);
    }

    function test_value_updates_keep_the_delegates() {
        const compact = make({
            items: [item("a", 72, "good")]
        });
        const before = labels(compact)[0];
        compact.items = [item("a", 12, "critical")];
        const after = labels(compact)[0];
        verify(before === after);
        compare(after.text, "12%");
    }

    function test_icon_mode_shows_only_the_mark() {
        const compact = make({
            items: [],
            panelTone: "warning",
            display: Settings.parseDisplay({
                panel_mode: "icon"
            })
        });
        verify(compact.markShown);
        compare(labels(compact).length, 0);
        compare(compact.markTone, "warning");
    }

    function test_thin_vertical_panel_shows_the_worst_item() {
        const compact = make({
            width: 30,
            vertical: true,
            items: [item("a", 72, "good"), item("b", 8, "critical")],
            display: Settings.parseDisplay({
                panel_mode: "several"
            })
        });
        compare(labels(compact).map(label => label.text), ["8%"]);
    }

    name: "Compact"
    width: 300
    height: 60

    Component {
        id: compactComponent

        CompactRepresentation {
            width: 300
            height: 30
            reducedMotion: true
        }
    }
}
