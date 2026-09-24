pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Format.js" as Format
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens
import "logic/Trend.js" as Trend

Item {
    id: trend

    required property var usage
    required property string lang
    readonly property var days: Trend.lastDays(usage.daily)
    readonly property int peak: Trend.peakOf(days)
    readonly property real stripHeight: Metrics.trendHeight(Kirigami.Units)
    readonly property real barWidth: Kirigami.Units.smallSpacing
    readonly property real barGap: Metrics.hairline(Kirigami.Units)
    readonly property int hoveredIndex: stripHover.shown ? Trend.indexAt(stripHover.point.position.x, barWidth + barGap, days.length) : -1
    readonly property string tipText: hoveredIndex >= 0 ? Format.dayTooltip(lang, days[hoveredIndex]) : Trend.peakDescription(lang, days)

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
            text: I18n.tr(trend.lang, "Usage Trend")
            elide: Text.ElideRight
        }

        Row {
            id: strip

            Layout.alignment: Qt.AlignVCenter
            Layout.preferredHeight: trend.stripHeight
            spacing: trend.barGap

            Repeater {
                model: trend.days

                Rectangle {
                    required property var modelData
                    required property int index
                    readonly property real share: Trend.barShare(modelData.totalTokens, trend.peak)

                    anchors.bottom: parent.bottom
                    width: trend.barWidth
                    height: share > 0 ? Math.round(trend.stripHeight * share) : Kirigami.Units.smallSpacing / 2
                    radius: Metrics.hairline(Kirigami.Units)
                    color: Tokens.toneColor(Kirigami.Theme, "good")
                    opacity: trend.hoveredIndex < 0 || trend.hoveredIndex === index ? 1 : 0.4

                    Behavior on opacity {
                        NumberAnimation {
                            duration: Kirigami.Units.shortDuration
                        }
                    }
                }
            }

            PointerHover {
                id: stripHover
            }

            PlainToolTip {
                text: trend.tipText
                visible: stripHover.shown && trend.tipText !== ""
                delay: 0
            }
        }
    }
}
