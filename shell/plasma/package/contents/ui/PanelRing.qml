import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import "logic/Motion.js" as Motion
import "logic/Tokens.js" as Tokens

Shape {
    id: ring

    property real size: Kirigami.Units.iconSizes.small
    property real fraction: 0
    property string tone: "neutral"
    property bool reducedMotion: false
    property real shownFraction: fraction
    readonly property bool critical: tone === "critical"
    readonly property real lineWidth: Math.max(2, size * 0.2)
    readonly property real arcRadius: (size - lineWidth) / 2
    property color arcColor: tone === "warning" || tone === "critical" ? Tokens.toneColor(Kirigami.Theme, tone) : Kirigami.Theme.textColor

    function pulse() {
        if (critical && visible && Motion.enabled(Kirigami.Units, reducedMotion))
            pulseAnimation.restart();
    }

    implicitWidth: size
    implicitHeight: size
    preferredRendererType: Shape.CurveRenderer
    onCriticalChanged: pulse()
    onFractionChanged: pulse()

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

    SequentialAnimation {
        id: pulseAnimation

        loops: 3
        onStopped: ring.opacity = 1

        NumberAnimation {
            target: ring
            property: "opacity"
            to: 0.45
            duration: Motion.pulseDuration(Kirigami.Units) / 2
            easing.type: Easing.InOutSine
        }

        NumberAnimation {
            target: ring
            property: "opacity"
            to: 1
            duration: Motion.pulseDuration(Kirigami.Units) / 2
            easing.type: Easing.InOutSine
        }
    }

    Behavior on shownFraction {
        NumberAnimation {
            duration: Kirigami.Units.longDuration
            easing.type: Easing.OutCubic
        }
    }

    Behavior on arcColor {
        ColorAnimation {
            duration: Kirigami.Units.longDuration
        }
    }
}
