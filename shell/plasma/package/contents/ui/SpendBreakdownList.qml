pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Options.js" as Options
import "logic/Providers.js" as Providers
import "logic/SpendBreakdown.js" as SpendBreakdown
import "logic/Tokens.js" as Tokens

ColumnLayout {
    id: list

    required property var period
    required property string mode
    required property bool byTokens
    required property string lang
    readonly property var model: SpendBreakdown.breakdown(lang, period, mode, byTokens)
    readonly property bool dark: Tokens.isDark(Kirigami.Theme)
    readonly property real barHeight: Math.max(2, Math.round(Kirigami.Units.smallSpacing * 0.75))

    signal modeSelected(string key)

    function segmentColor(provider) {
        return provider === "" ? Tokens.tertiaryText(Kirigami.Theme) : Providers.ringColor(provider, dark);
    }

    objectName: "spendBreakdown"
    Layout.fillWidth: true
    Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
    Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
    spacing: Kirigami.Units.largeSpacing

    Rectangle {
        Layout.fillWidth: true
        implicitHeight: Metrics.hairline(Kirigami.Units)
        color: Tokens.separator(Kirigami.Theme)
    }

    RowLayout {
        spacing: Kirigami.Units.largeSpacing

        SegmentedControl {
            objectName: "breakdownControl"
            implicitWidth: Kirigami.Units.gridUnit * 8
            options: Options.spendBreakdownOptions(list.lang)
            current: list.mode
            onSelected: value => list.modeSelected(value)
        }

        TextLabel {
            Layout.fillWidth: true
            role: "caption"
            emphasis: "secondary"
            horizontalAlignment: Text.AlignRight
            text: list.model?.caption ?? ""
            elide: Text.ElideLeft
        }
    }

    Repeater {
        model: list.model?.rows ?? []

        ColumnLayout {
            id: entry

            required property var modelData

            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            RowLayout {
                spacing: Kirigami.Units.mediumSpacing

                Rectangle {
                    visible: !entry.modelData.folder
                    Layout.alignment: Qt.AlignVCenter
                    implicitWidth: Kirigami.Units.largeSpacing
                    implicitHeight: implicitWidth
                    radius: width / 2
                    color: list.segmentColor(entry.modelData.provider)
                }

                Kirigami.Icon {
                    visible: entry.modelData.folder
                    Layout.alignment: Qt.AlignVCenter
                    implicitWidth: Metrics.caretIcon(Kirigami.Units)
                    implicitHeight: implicitWidth
                    source: "folder"
                    isMask: true
                    color: Tokens.secondaryText(Kirigami.Theme)
                }

                TextLabel {
                    Layout.fillWidth: entry.modelData.detail === ""
                    font.features: ({})
                    text: entry.modelData.name
                    elide: Text.ElideMiddle
                }

                TextLabel {
                    visible: entry.modelData.detail !== ""
                    Layout.fillWidth: true
                    role: "caption"
                    emphasis: "secondary"
                    text: `· ${entry.modelData.detail}`
                    elide: Text.ElideRight
                }

                TextLabel {
                    role: "caption"
                    emphasis: "secondary"
                    text: entry.modelData.share
                }

                TextLabel {
                    Layout.minimumWidth: Kirigami.Units.gridUnit * 2.5
                    horizontalAlignment: Text.AlignRight
                    weight: Font.Medium
                    text: entry.modelData.value
                }
            }

            Rectangle {
                id: track

                Layout.fillWidth: true
                implicitHeight: list.barHeight
                radius: height / 2
                color: Tokens.track(Kirigami.Theme)

                Row {
                    height: parent.height
                    spacing: Metrics.hairline(Kirigami.Units)

                    Repeater {
                        model: entry.modelData.segments

                        Rectangle {
                            required property var modelData

                            height: track.height
                            width: Math.max(Metrics.hairline(Kirigami.Units), track.width * modelData.fraction)
                            radius: height / 2
                            color: list.segmentColor(modelData.provider)
                        }
                    }
                }
            }
        }
    }
}
