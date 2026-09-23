pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Format.js" as Format
import "logic/Metrics.js" as Metrics
import "logic/Providers.js" as Providers
import "logic/Spend.js" as Spend
import "logic/Tokens.js" as Tokens

ColumnLayout {
    id: legend

    required property var period
    required property string periodKey
    required property string lang

    spacing: Math.round(Kirigami.Units.smallSpacing / 2)

    Repeater {
        model: legend.period.providers

        Item {
            id: entry

            required property var modelData

            Layout.fillWidth: true
            implicitHeight: row.implicitHeight + Kirigami.Units.smallSpacing * 1.5

            Rectangle {
                anchors.fill: parent
                radius: Metrics.chipRadius(Kirigami.Units)
                color: Tokens.chip(Kirigami.Theme)
                opacity: tip.hovered ? 1 : 0

                Behavior on opacity {
                    NumberAnimation {
                        duration: Kirigami.Units.shortDuration
                    }
                }
            }

            RowLayout {
                id: row

                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: Kirigami.Units.smallSpacing
                anchors.rightMargin: Kirigami.Units.smallSpacing
                spacing: Kirigami.Units.mediumSpacing

                Rectangle {
                    Layout.alignment: Qt.AlignVCenter
                    implicitWidth: Kirigami.Units.largeSpacing
                    implicitHeight: implicitWidth
                    radius: width / 2
                    color: Providers.ringColor(entry.modelData.provider, Tokens.isDark(Kirigami.Theme))
                }

                TextLabel {
                    Layout.fillWidth: true
                    text: entry.modelData.providerName
                    wrapMode: Text.Wrap
                    maximumLineCount: 2
                }

                TextLabel {
                    Layout.alignment: Qt.AlignVCenter
                    weight: Font.Medium
                    text: Format.usd(entry.modelData.costMicros)
                }
            }

            ModelTip {
                id: tip

                lang: legend.lang
                title: Spend.breakdownTitle(legend.lang, legend.periodKey, entry.modelData)
                totals: entry.modelData
                fallback: Format.spendTooltip(legend.lang, entry.modelData)
            }
        }
    }
}
