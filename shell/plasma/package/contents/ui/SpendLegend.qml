pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Format.js" as Format
import "logic/Providers.js" as Providers

ColumnLayout {
    id: legend

    required property var period

    spacing: Math.round(Kirigami.Units.smallSpacing * 1.75)

    Repeater {
        model: legend.period.providers

        RowLayout {
            id: entry

            required property var modelData

            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            Rectangle {
                implicitWidth: Kirigami.Units.largeSpacing
                implicitHeight: implicitWidth
                radius: width / 2
                color: Providers.providerInfo(entry.modelData.provider).ringColor
            }

            TextLabel {
                Layout.fillWidth: true
                text: Providers.providerInfo(entry.modelData.provider).name
                elide: Text.ElideRight
            }

            TextLabel {
                weight: Font.Medium
                text: Format.usd(entry.modelData.costMicros)

                HoverTip {
                    text: Format.spendTooltip(entry.modelData)
                }
            }
        }
    }
}
