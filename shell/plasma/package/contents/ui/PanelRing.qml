import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import "logic/Tokens.js" as Tokens

Shape {
    id: ring

    property real size: Kirigami.Units.iconSizes.small
    property real fraction: 0
    property string tone: "neutral"
    readonly property real lineWidth: Math.max(2, size * 0.2)
    readonly property real arcRadius: (size - lineWidth) / 2

    function toneFill(tone) {
        return tone === "warning" || tone === "critical" ? Tokens.toneColor(Kirigami.Theme, tone) : Kirigami.Theme.textColor;
    }

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
        strokeColor: ring.fraction > 0 ? ring.toneFill(ring.tone) : "transparent"
        strokeWidth: ring.lineWidth
        fillColor: "transparent"
        capStyle: ShapePath.RoundCap

        PathAngleArc {
            centerX: ring.size / 2
            centerY: ring.size / 2
            radiusX: ring.arcRadius
            radiusY: ring.arcRadius
            startAngle: -90
            sweepAngle: Math.min(1, Math.max(0, ring.fraction)) * 360
        }
    }
}
