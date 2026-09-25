import QtQuick
import QtTest
import "../package/contents/ui/logic/PanelLayout.js" as PanelLayout
import "../package/contents/ui/logic/Settings.js" as Settings

TestCase {
    name: "PanelLayout"

    readonly property var units: ({
            gridUnit: 18,
            smallSpacing: 4,
            longDuration: 200
        })
    readonly property var theme: ({
            textColor: "#232629",
            neutralTextColor: "#f67400",
            negativeTextColor: "#da4453",
            highlightColor: "#3daee9"
        })

    function display(mode, indicator, label) {
        return Settings.parseDisplay({
            panel_mode: mode,
            panel_indicator: indicator,
            panel_label: label
        });
    }

    function item(accountId, windowId, tone) {
        return {
            accountId,
            windowId,
            tone,
            valuePercent: 40,
            evenPacePercent: 30
        };
    }

    function test_mark_shows_for_icon_mode_empty_items_and_nothing_to_draw() {
        const one = [item("a", "session", "good")];
        verify(PanelLayout.showsMark(one, display("icon", "ring", "percent")));
        verify(PanelLayout.showsMark([], display("headline", "ring", "percent")));
        verify(PanelLayout.showsMark(one, display("headline", "none", "none")));
        verify(!PanelLayout.showsMark(one, display("headline", "ring", "none")));
        verify(!PanelLayout.showsMark(one, display("several", "none", "percent")));
    }

    function test_mark_tone_comes_from_panel_tone_in_icon_mode_only() {
        compare(PanelLayout.markTone(display("icon", "ring", "percent"), "warning"), "warning");
        compare(PanelLayout.markTone(display("icon", "ring", "percent"), null), "neutral");
        compare(PanelLayout.markTone(display("headline", "ring", "percent"), "critical"), "neutral");
    }

    function test_parts_data() {
        return [
            {
                tag: "headline percent is today's ring and value",
                display: display("headline", "ring", "percent"),
                vertical: false,
                parts: {
                    logo: false,
                    letter: false,
                    indicator: "ring",
                    value: true,
                    tintedValue: false
                }
            },
            {
                tag: "headline window adds logo and letter",
                display: display("headline", "none", "window"),
                vertical: false,
                parts: {
                    logo: true,
                    letter: true,
                    indicator: "none",
                    value: true,
                    tintedValue: true
                }
            },
            {
                tag: "several shows logos",
                display: display("several", "bar", "percent"),
                vertical: false,
                parts: {
                    logo: true,
                    letter: true,
                    indicator: "bar",
                    value: true,
                    tintedValue: false
                }
            },
            {
                tag: "vertical drops the letter",
                display: display("several", "none", "percent"),
                vertical: true,
                parts: {
                    logo: true,
                    letter: false,
                    indicator: "none",
                    value: true,
                    tintedValue: true
                }
            },
            {
                tag: "ring only",
                display: display("headline", "ring", "none"),
                vertical: false,
                parts: {
                    logo: false,
                    letter: false,
                    indicator: "ring",
                    value: false,
                    tintedValue: false
                }
            }
        ];
    }

    function test_parts(data) {
        compare(PanelLayout.parts(data.display, data.vertical), data.parts);
    }

    function test_thin_vertical_panel_keeps_only_the_worst_item() {
        const items = [item("a", "session", "good"), item("b", "weekly", "warning"), item("c", "weekly", "critical")];
        compare(PanelLayout.thinLimit(units), 40);
        compare(PanelLayout.shownItems(items, true, 32, units), [items[2]]);
        compare(PanelLayout.shownItems(items.slice(0, 2), true, 32, units), [items[1]]);
        const calm = [item("a", "session", "good"), item("b", "weekly", "neutral")];
        compare(PanelLayout.shownItems(calm, true, 32, units), [calm[0]]);
        compare(PanelLayout.shownItems(items, true, 48, units), items);
        compare(PanelLayout.shownItems(items, false, 24, units), items);
    }

    function test_keys_change_only_with_the_item_set() {
        const before = [item("a", "session", "good"), item("b", "weekly", "warning")];
        const after = [Object.assign({}, before[0], {
                valuePercent: 10,
                tone: "critical"
            }), before[1]];
        compare(PanelLayout.keys(before), PanelLayout.keys(after));
        compare(PanelLayout.keyList(PanelLayout.keys(before)).length, 2);
        compare(PanelLayout.keyList(""), []);
        verify(PanelLayout.keys(before) !== PanelLayout.keys([before[1], before[0]]));
    }

    function test_item_at_never_returns_undefined() {
        compare(PanelLayout.itemAt([], 0).tone, "neutral");
        compare(PanelLayout.itemAt([item("a", "session", "good")], 0).accountId, "a");
    }

    function test_bar_fraction_and_tick_follow_the_reading() {
        const shown = item("a", "session", "good");
        compare(PanelLayout.fraction(shown), 0.4);
        compare(PanelLayout.fraction(Object.assign({}, shown, {
            valuePercent: 130
        })), 1);
        fuzzyCompare(PanelLayout.tickPosition(shown, "left"), 0.7, 1e-9);
        fuzzyCompare(PanelLayout.tickPosition(shown, "used"), 0.3, 1e-9);
        compare(PanelLayout.tickPosition(Object.assign({}, shown, {
            evenPacePercent: null
        }), "left"), null);
    }

    function test_value_text_and_count() {
        compare(PanelLayout.valueText(item("a", "session", "good")), "40%");
        verify(PanelLayout.showsCount({
            combined: true,
            accountCount: 2
        }));
        verify(!PanelLayout.showsCount({
            combined: false,
            accountCount: 1
        }));
    }

    function test_colors_follow_the_panel_foreground() {
        compare(PanelLayout.toneColor(theme, "good"), theme.textColor);
        compare(PanelLayout.toneColor(theme, "neutral"), theme.textColor);
        compare(PanelLayout.toneColor(theme, "warning"), theme.neutralTextColor);
        compare(PanelLayout.toneColor(theme, "critical"), theme.negativeTextColor);
        compare(PanelLayout.valueColor(theme, "critical", false), theme.textColor);
        compare(PanelLayout.valueColor(theme, "critical", true), theme.negativeTextColor);
    }

    function test_pulse_only_for_critical_with_motion() {
        verify(PanelLayout.pulses("critical", units, false));
        verify(!PanelLayout.pulses("warning", units, false));
        verify(!PanelLayout.pulses("critical", units, true));
        verify(!PanelLayout.pulses("critical", Object.assign({}, units, {
            longDuration: 0
        }), false));
        compare(PanelLayout.pulsePeriod(units), 2000);
    }
}
