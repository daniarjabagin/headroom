pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import "logic/Format.js" as Format
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Spend.js" as Spend

Item {
    id: donut

    required property var period
    required property string lang
    property real progress: 1
    readonly property real size: Metrics.donutSize(Kirigami.Units)
    readonly property real holeRatio: 0.618
    readonly property real thickness: size / 2 * (1 - holeRatio)
    readonly property real arcRadius: size / 2 - thickness / 2
    readonly property real gapDegrees: Metrics.hairline(Kirigami.Units) * 1.5 / (Math.PI * arcRadius) * 180

    implicitWidth: size
    implicitHeight: size

    Repeater {
        model: Spend.slices(donut.period.providers, donut.gapDegrees)

        Shape {
            id: slice

            required property var modelData

            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: slice.modelData.color
                strokeWidth: donut.thickness
                fillColor: "transparent"
                capStyle: ShapePath.FlatCap

                PathAngleArc {
                    centerX: donut.size / 2
                    centerY: donut.size / 2
                    radiusX: donut.arcRadius
                    radiusY: donut.arcRadius
                    startAngle: slice.modelData.start
                    sweepAngle: Spend.revealed(slice.modelData, donut.progress)
                }
            }
        }
    }

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 0
        opacity: donut.progress

        TextLabel {
            Layout.alignment: Qt.AlignHCenter
            role: "label"
            text: Format.ringUsd(donut.period.costMicros)
        }

        TextLabel {
            Layout.alignment: Qt.AlignHCenter
            role: "micro"
            weight: Font.Medium
            emphasis: "secondary"
            text: I18n.tr(donut.lang, "dollars")
        }
    }
}
