import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import "logic/Motion.js" as Motion
import "logic/PanelLayout.js" as PanelLayout
import "logic/Tokens.js" as Tokens

Shape {
    id: ring

    property real size: Kirigami.Units.iconSizes.small
    property real fraction: 0
    property string tone: "neutral"
    property bool reducedMotion: false
    property real shownFraction: fraction
    readonly property real lineWidth: Math.max(2, size * 0.2)
    readonly property real arcRadius: (size - lineWidth) / 2
    readonly property int motionDuration: Motion.enabled(Kirigami.Units, reducedMotion) ? Kirigami.Units.longDuration : 0
    property color arcColor: PanelLayout.toneColor(Kirigami.Theme, tone)

    implicitWidth: size
    implicitHeight: size
    preferredRendererType: Shape.CurveRenderer

    ShapePath {
        strokeColor: Tokens.panelTrack(Kirigami.Theme)
        strokeWidth: ring.lineWidth
        fillColor: "transparent"

        PathAngleArc {
            centerX: ring.size / 2
            centerY: ring.size / 2
            radiusX: ring.arcRadius
            radiusY: ring.arcRadius
            startAngle: 0
            sweepAngle: 360
        }
    }

    ShapePath {
        strokeColor: ring.shownFraction > 0 ? ring.arcColor : "transparent"
        strokeWidth: ring.lineWidth
        fillColor: "transparent"
        capStyle: ShapePath.RoundCap

        PathAngleArc {
            centerX: ring.size / 2
            centerY: ring.size / 2
            radiusX: ring.arcRadius
            radiusY: ring.arcRadius
            startAngle: -90
            sweepAngle: Math.min(1, Math.max(0, ring.shownFraction)) * 360
        }
    }

    Behavior on shownFraction {
        NumberAnimation {
            duration: ring.motionDuration
            easing.type: Easing.OutCubic
        }
    }

    Behavior on arcColor {
        ColorAnimation {
            duration: ring.motionDuration
        }
    }
}
