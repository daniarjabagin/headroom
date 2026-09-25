pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Sector.js" as Sector
import "logic/Spend.js" as Spend
import "logic/SpendUnits.js" as SpendUnits
import "logic/Tokens.js" as Tokens

Item {
    id: donut

    required property var period
    property real progress: 1
    property string unit: "cost"
    property string lang: "en"
    property real size: Metrics.donutSize(Kirigami.Units)
    readonly property string caption: SpendUnits.ringCaption(lang, unit)
    readonly property real holeRatio: 0.618
    readonly property bool gapped: period.providers.length > 1
    readonly property var geometry: Sector.geometry(size, holeRatio, gapped ? Metrics.donutGap(Kirigami.Units) : 0)
    readonly property var slices: Spend.slices(period.providers, Sector.minSweep(geometry), Tokens.isDark(Kirigami.Theme), SpendUnits.byTokens(unit))

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

    Column {
        anchors.centerIn: parent
        opacity: donut.progress

        TextLabel {
            objectName: "ringValue"
            anchors.horizontalCenter: parent.horizontalCenter
            role: "label"
            step: donut.size < Metrics.donutSize(Kirigami.Units) ? 1 : 0
            text: SpendUnits.ringValue(donut.period, donut.unit)
        }

        TextLabel {
            visible: donut.caption !== ""
            anchors.horizontalCenter: parent.horizontalCenter
            role: "micro"
            emphasis: "secondary"
            weight: Font.Medium
            text: donut.caption
        }
    }
}
