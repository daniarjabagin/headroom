pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import "logic/Format.js" as Format
import "logic/Metrics.js" as Metrics
import "logic/Sector.js" as Sector
import "logic/Spend.js" as Spend

Item {
    id: donut

    required property var period
    property real progress: 1
    readonly property real size: Metrics.donutSize(Kirigami.Units)
    readonly property real holeRatio: 0.618
    readonly property bool gapped: period.providers.length > 1
    readonly property var geometry: Sector.geometry(size, holeRatio, gapped ? Metrics.donutGap(Kirigami.Units) : 0)
    readonly property var slices: Spend.slices(period.providers, Sector.minSweep(geometry))

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
                fillColor: slice.target.color
                fillRule: ShapePath.OddEvenFill
                strokeWidth: -1

                PathSvg {
                    path: Sector.slicePath(donut.geometry, slice.start, slice.shown)
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
