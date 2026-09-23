pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import "logic/Format.js" as Format
import "logic/Metrics.js" as Metrics
import "logic/Spend.js" as Spend

Item {
    id: donut

    required property var period
    property real progress: 1
    readonly property real size: Metrics.donutSize(Kirigami.Units)
    readonly property real holeRatio: 0.618
    readonly property real thickness: size / 2 * (1 - holeRatio)
    readonly property real arcRadius: size / 2 - thickness / 2
    readonly property real gapDegrees: degreesFor(Metrics.donutGap(Kirigami.Units))
    readonly property real capDegrees: degreesFor(thickness / 2)
    readonly property var slices: Spend.slices(period.providers, gapDegrees, capDegrees)

    function degreesFor(length) {
        return length / arcRadius * 180 / Math.PI;
    }

    implicitWidth: size
    implicitHeight: size

    Repeater {
        model: donut.slices.length

        Shape {
            id: slice

            required property int index
            readonly property var target: donut.slices[index] ?? donut.slices[donut.slices.length - 1]
            property real start: target.start
            property real sweep: target.sweep
            readonly property real shown: Spend.revealed({
                start: slice.start,
                sweep: slice.sweep
            }, donut.progress)

            anchors.fill: parent
            visible: shown > 0
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: slice.target.color
                strokeWidth: donut.thickness
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap

                PathAngleArc {
                    centerX: donut.size / 2
                    centerY: donut.size / 2
                    radiusX: donut.arcRadius
                    radiusY: donut.arcRadius
                    startAngle: slice.start
                    sweepAngle: Math.max(Spend.DOT_SWEEP, slice.shown)
                }
            }

            Behavior on start {
                NumberAnimation {
                    duration: Kirigami.Units.longDuration
                    easing.type: Easing.OutCubic
                }
            }

            Behavior on sweep {
                NumberAnimation {
                    duration: Kirigami.Units.longDuration
                    easing.type: Easing.OutCubic
                }
            }
        }
    }

    TextLabel {
        anchors.centerIn: parent
        opacity: donut.progress
        role: "label"
        text: Format.ringUsd(donut.period.costMicros)
    }
}
