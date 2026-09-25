pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics

Item {
    id: meter

    property var segments: []
    property real progress: 1
    property real barHeight: Metrics.meterHeight(Kirigami.Units)
    readonly property real gap: Metrics.hairline(Kirigami.Units) * 2

    Layout.fillWidth: true
    implicitHeight: barHeight

    Row {
        anchors.fill: parent
        spacing: meter.gap

        Repeater {
            model: meter.segments.length

            Meter {
                required property int index
                readonly property var segment: meter.segments[index]

                width: meter.segments.length > 0 ? (meter.width - meter.gap * (meter.segments.length - 1)) / meter.segments.length : 0
                height: meter.height
                barHeight: meter.barHeight
                fraction: segment.fraction
                progress: meter.progress
                tone: segment.tone
                tick: segment.tick
            }
        }
    }
}
