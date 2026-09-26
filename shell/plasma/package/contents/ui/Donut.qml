pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/RingFit.js" as RingFit
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
    property bool animated: true
    property real size: Metrics.donutSize(Kirigami.Units)
    readonly property string caption: SpendUnits.ringCaption(lang, unit)
    readonly property real holeRatio: 0.618
    readonly property bool gapped: period.providers.length > 1
    readonly property var geometry: Sector.geometry(size, holeRatio, gapped ? Metrics.donutGap(Kirigami.Units) : 0)
    readonly property real fit: RingFit.scale(geometry.inner * 2, Kirigami.Units.smallSpacing, Math.max(valueMetrics.advanceWidth, caption === "" ? 0 : captionMetrics.advanceWidth), valueMetrics.height + (caption === "" ? 0 : captionMetrics.height))
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
                enabled: donut.animated

                NumberAnimation {
                    duration: Kirigami.Units.longDuration
                    easing.type: Easing.OutCubic
                }
            }

            Behavior on sweep {
                enabled: donut.animated

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
            id: valueLabel

            objectName: "ringValue"
            anchors.horizontalCenter: parent.horizontalCenter
            role: "label"
            step: donut.size < Metrics.donutSize(Kirigami.Units) ? 1 : 0
            fit: donut.fit
            text: SpendUnits.ringValue(donut.period, donut.unit)
        }

        TextLabel {
            id: captionLabel

            objectName: "ringCaption"
            visible: donut.caption !== ""
            anchors.horizontalCenter: parent.horizontalCenter
            role: "micro"
            emphasis: "secondary"
            weight: Font.Medium
            fit: donut.fit
            text: donut.caption
        }
    }

    TextMetrics {
        id: valueMetrics

        font.family: valueLabel.font.family
        font.pointSize: valueLabel.pointSizeFor(valueLabel.role) - valueLabel.step
        font.weight: valueLabel.weight
        font.features: {
            "tnum": 1
        }
        text: valueLabel.text
    }

    TextMetrics {
        id: captionMetrics

        font.family: captionLabel.font.family
        font.pointSize: captionLabel.pointSizeFor(captionLabel.role) - captionLabel.step
        font.weight: captionLabel.weight
        font.features: {
            "tnum": 1
        }
        text: captionLabel.text
    }
}
