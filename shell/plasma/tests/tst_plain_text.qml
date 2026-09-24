pragma ComponentBehavior: Bound
import QtQuick
import QtTest
import "../package/contents/ui"
import "../package/contents/ui/logic/Settings.js" as Settings

TestCase {
    id: suite

    readonly property string markup: "<img src=\"https://example.invalid/x.png\"><b>bold</b>"

    function test_compact_labels_render_plain_text() {
        const compact = createTemporaryObject(compactComponent, suite);
        verify(compact !== null);
        const windowLabel = findChild(compact, "compactWindowLabel");
        const percentLabel = findChild(compact, "compactPercentLabel");
        verify(windowLabel !== null);
        verify(percentLabel !== null);
        compare(windowLabel.text, suite.markup);
        compare(windowLabel.textFormat, Text.PlainText);
        compare(percentLabel.textFormat, Text.PlainText);
    }

    function test_tooltips_render_plain_text() {
        const tip = createTemporaryObject(tipComponent, suite);
        verify(tip !== null);
        const label = tip.contentItem;
        compare(label.objectName, "plainToolTipLabel");
        compare(label.text, suite.markup);
        compare(label.textFormat, Text.PlainText);
    }

    name: "PlainText"
    width: 200
    height: 60

    Component {
        id: compactComponent

        CompactRepresentation {
            width: 200
            height: 30
            display: Settings.parseDisplay({
                panel_label: "window"
            })
            headline: ({
                    provider: "codex",
                    windowId: "custom-window",
                    windowLabel: suite.markup,
                    remainingPercent: 40,
                    usedPercent: 60,
                    tone: "good"
                })
        }
    }

    Component {
        id: tipComponent

        PlainToolTip {
            text: suite.markup
        }
    }
}
