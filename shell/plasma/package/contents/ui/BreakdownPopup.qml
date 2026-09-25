pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/Breakdown.js" as Breakdown
import "logic/FormatSpend.js" as FormatSpend
import "logic/Metrics.js" as Metrics
import "logic/Providers.js" as Providers
import "logic/Tokens.js" as Tokens

PointerHover {
    id: hover

    property string lang: "en"
    property string title: ""
    property var totals: null
    property string fallback: ""
    readonly property bool loaded: shown && totals !== null
    readonly property var rows: loaded ? Breakdown.popoverRows(lang, totals) : []
    readonly property var notes: loaded ? Breakdown.popoverNotes(lang, totals) : []
    readonly property color barColor: totals === null ? Kirigami.Theme.highlightColor : Providers.ringColor(totals.provider, Tokens.isDark(Kirigami.Theme))
    readonly property real barHeight: Math.max(2, Math.round(Kirigami.Units.smallSpacing * 0.75))
    readonly property real figuresWidth: Math.round(Kirigami.Units.gridUnit * 6.5)

    property PlasmaComponents3.ToolTip tip: PlasmaComponents3.ToolTip {
        objectName: "breakdownPopup"
        parent: hover.parent
        delay: Kirigami.Units.veryLongDuration
        visible: hover.shown && (hover.rows.length > 0 || hover.fallback !== "")

        contentItem: ColumnLayout {
            implicitWidth: hover.rows.length > 0 ? Math.round(Kirigami.Units.gridUnit * 13.5) : fallbackLabel.implicitWidth
            spacing: Math.round(Kirigami.Units.smallSpacing * 1.75)

            TextLabel {
                id: fallbackLabel

                visible: hover.rows.length === 0
                text: hover.fallback
            }

            RowLayout {
                visible: hover.rows.length > 0
                Layout.fillWidth: true
                spacing: Kirigami.Units.largeSpacing

                TextLabel {
                    Layout.fillWidth: true
                    role: "caption"
                    weight: Font.DemiBold
                    text: hover.title
                    elide: Text.ElideRight
                }

                TextLabel {
                    role: "caption"
                    weight: Font.DemiBold
                    text: hover.totals === null ? "" : FormatSpend.usd(hover.totals.costMicros)
                }
            }

            Rectangle {
                visible: hover.rows.length > 0
                Layout.fillWidth: true
                implicitHeight: Metrics.hairline(Kirigami.Units)
                color: Tokens.separator(Kirigami.Theme)
            }

            Repeater {
                model: hover.rows

                ColumnLayout {
                    id: modelRow

                    required property var modelData

                    Layout.fillWidth: true
                    spacing: Math.round(Kirigami.Units.smallSpacing * 0.75)

                    RowLayout {
                        spacing: Kirigami.Units.mediumSpacing

                        TextLabel {
                            Layout.fillWidth: modelRow.modelData.detail === ""
                            role: "caption"
                            font.features: ({})
                            text: modelRow.modelData.partial ? `${modelRow.modelData.name} *` : modelRow.modelData.name
                            elide: Text.ElideRight
                        }

                        TextLabel {
                            visible: modelRow.modelData.detail !== ""
                            role: "caption"
                            emphasis: "secondary"
                            text: `· ${modelRow.modelData.detail}`
                        }

                        Item {
                            visible: modelRow.modelData.detail !== ""
                            Layout.fillWidth: true
                        }

                        TextLabel {
                            role: "caption"
                            weight: Font.DemiBold
                            text: modelRow.modelData.cost
                        }
                    }

                    RowLayout {
                        spacing: Kirigami.Units.largeSpacing

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignVCenter
                            implicitHeight: hover.barHeight
                            radius: height / 2
                            color: Tokens.track(Kirigami.Theme)

                            Rectangle {
                                height: parent.height
                                radius: height / 2
                                width: modelRow.modelData.fraction > 0 ? Math.max(height, parent.width * modelRow.modelData.fraction) : 0
                                color: hover.barColor
                            }
                        }

                        TextLabel {
                            Layout.preferredWidth: hover.figuresWidth
                            role: "caption"
                            emphasis: "secondary"
                            horizontalAlignment: Text.AlignRight
                            text: modelRow.modelData.figures
                        }
                    }
                }
            }

            Rectangle {
                visible: hover.rows.length > 0
                Layout.fillWidth: true
                implicitHeight: Metrics.hairline(Kirigami.Units)
                color: Tokens.separator(Kirigami.Theme)
            }

            TextLabel {
                visible: hover.rows.length > 0
                Layout.fillWidth: true
                role: "micro"
                emphasis: "secondary"
                wrapMode: Text.Wrap
                text: hover.notes.join("\n")
            }
        }
    }
}
