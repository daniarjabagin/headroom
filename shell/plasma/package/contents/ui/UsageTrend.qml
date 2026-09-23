pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens
import "logic/Trend.js" as Trend
import org.kde.kirigami as Kirigami

Item {
    id: trend

    required property var usage
    readonly property var days: Trend.lastDays(usage.daily)
    readonly property int peak: Trend.peakOf(days)
    readonly property real stripHeight: Metrics.trendHeight(Kirigami.Units)

    Layout.fillWidth: true
    implicitHeight: content.implicitHeight + Metrics.textRowPadding(Kirigami.Units) * 2

    RowLayout {
        id: content

        anchors.fill: parent
        anchors.leftMargin: Metrics.rowInset(Kirigami.Units)
        anchors.rightMargin: Metrics.rowInset(Kirigami.Units)
        spacing: Kirigami.Units.largeSpacing

        TextLabel {
            Layout.fillWidth: true
            weight: Font.DemiBold
            text: "Usage Trend"
        }

        Row {
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredHeight: trend.stripHeight
            spacing: Metrics.hairline(Kirigami.Units)

            Repeater {
                model: trend.days

                Rectangle {
                    required property var modelData
                    readonly property real share: Trend.barShare(modelData.totalTokens, trend.peak)

                    anchors.bottom: parent.bottom
                    width: Kirigami.Units.smallSpacing
                    height: share > 0 ? Math.round(trend.stripHeight * share) : Kirigami.Units.smallSpacing / 2
                    radius: Metrics.hairline(Kirigami.Units)
                    color: Tokens.toneColor(Kirigami.Theme, "good")
                }
            }

            HoverTip {
                text: Trend.peakDescription(trend.days)
            }
        }
    }
}
