pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/Breakdown.js" as Breakdown
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

HoverHandler {
    id: hover

    property string lang: "en"
    property string title: ""
    property var totals: null
    property string fallback: ""
    readonly property var breakdown: totals === null ? null : Breakdown.modelBreakdown(lang, totals.models, totals.modelsOther)
    readonly property string partialMark: "*"

    function cells(rows) {
        const found = [];
        for (const row of rows)
            found.push({
                text: row.partial ? `${row.name} ${partialMark}` : row.name,
                role: "name"
            }, {
                text: row.tokens,
                role: "figure"
            }, {
                text: row.cost,
                role: "cost"
            });
        return found;
    }

    property PlasmaComponents3.ToolTip tip: PlasmaComponents3.ToolTip {
        parent: hover.parent
        visible: hover.hovered && (hover.breakdown !== null || hover.fallback !== "")

        contentItem: ColumnLayout {
            spacing: Kirigami.Units.smallSpacing

            TextLabel {
                visible: hover.breakdown === null
                text: hover.fallback
            }

            TextLabel {
                visible: hover.breakdown !== null && hover.title !== ""
                role: "label"
                text: hover.title
            }

            GridLayout {
                visible: hover.breakdown !== null
                columns: 3
                columnSpacing: Kirigami.Units.largeSpacing
                rowSpacing: Math.round(Kirigami.Units.smallSpacing / 2)

                Repeater {
                    model: hover.breakdown === null ? [] : hover.cells(hover.breakdown.rows)

                    TextLabel {
                        required property var modelData

                        Layout.fillWidth: modelData.role === "name"
                        Layout.alignment: modelData.role === "name" ? Qt.AlignLeft : Qt.AlignRight
                        emphasis: modelData.role === "figure" ? "secondary" : "primary"
                        weight: modelData.role === "cost" ? Font.Medium : Font.Normal
                        text: modelData.text
                    }
                }
            }

            Rectangle {
                visible: hover.breakdown !== null
                Layout.fillWidth: true
                implicitHeight: Metrics.hairline(Kirigami.Units)
                color: Tokens.separator(Kirigami.Theme)
            }

            TextLabel {
                visible: hover.breakdown !== null
                weight: Font.Medium
                text: hover.totals === null ? "" : Breakdown.totalLine(hover.lang, hover.totals)
            }

            TextLabel {
                visible: hover.breakdown?.partial ?? false
                role: "caption"
                emphasis: "secondary"
                text: `${hover.partialMark} ${I18n.tr(hover.lang, "Partly unpriced, cost leaves it out")}`
            }
        }
    }
}
