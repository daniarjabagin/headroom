import QtQuick
import QtTest
import "../package/contents/ui"

TestCase {
    id: suite

    property OptionCombo combo: null
    property var picks: []
    property int rebuilds: 0

    function refreshOptions(labelOfFive) {
        return [
            {
                value: 60,
                label: "Every minute"
            },
            {
                value: 300,
                label: labelOfFive
            }
        ];
    }

    function init() {
        picks = [];
        combo = createTemporaryObject(comboComponent, suite, {
            options: refreshOptions("Every 5 minutes"),
            value: 300
        }) as OptionCombo;
        rebuilds = 0;
        combo.modelChanged.connect(() => rebuilds += 1);
    }

    function test_starts_on_value() {
        compare(combo.count, 2);
        compare(combo.currentIndex, 1);
    }

    function test_equal_options_keep_model() {
        combo.options = refreshOptions("Every 5 minutes");
        wait(0);
        compare(rebuilds, 0);
        compare(combo.currentIndex, 1);
    }

    function test_changed_options_replace_model() {
        combo.options = refreshOptions("Every five minutes");
        tryCompare(suite, "rebuilds", 1);
        compare(combo.textAt(1), "Every five minutes");
        compare(combo.currentIndex, 1);
    }

    function test_value_moves_selection() {
        combo.value = 60;
        compare(combo.currentIndex, 0);
    }

    function test_activation_never_rebuilds_model() {
        let rebuildsDuringPick = -1;
        combo.picked.connect(value => {
            picks.push(value);
            combo.options = refreshOptions("Every five minutes");
            combo.value = value;
            rebuildsDuringPick = suite.rebuilds;
        });
        combo.currentIndex = 0;
        combo.activated(0);
        compare(picks, [60]);
        compare(rebuildsDuringPick, 0);
        tryCompare(suite, "rebuilds", 1);
        compare(combo.currentIndex, 0);
    }

    function test_activation_with_equal_options_keeps_model() {
        combo.picked.connect(value => {
            combo.options = refreshOptions("Every 5 minutes");
            combo.value = value;
        });
        combo.currentIndex = 0;
        combo.activated(0);
        wait(0);
        compare(rebuilds, 0);
        compare(combo.currentIndex, 0);
    }

    Component {
        id: comboComponent

        OptionCombo {}
    }
}
