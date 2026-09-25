import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

Rectangle {
    id: track

    property real fraction: 0
    property real progress: 1
    property string tone: "neutral"
    property var tick: null
    property bool animated: true
    property real barHeight: Metrics.meterHeight(Kirigami.Units)
    readonly property real shown: fraction * progress

    Layout.fillWidth: true
    implicitHeight: barHeight
    radius: height / 2
    color: Tokens.track(Kirigami.Theme)

    Rectangle {
        height: parent.height
        radius: height / 2
        width: track.shown > 0 ? Math.max(height, track.width * track.shown) : 0
        color: Tokens.toneColor(Kirigami.Theme, track.tone)

        Behavior on width {
            enabled: track.animated && track.progress >= 1

            NumberAnimation {
                duration: Kirigami.Units.longDuration
                easing.type: Easing.OutCubic
            }
        }

        Behavior on color {
            enabled: track.animated

            ColorAnimation {
                duration: Kirigami.Units.longDuration
            }
        }
    }

    Rectangle {
        readonly property real overhang: Metrics.tickOverhang(Kirigami.Units)

        visible: track.tick !== null
        opacity: track.progress
        width: overhang
        height: track.height + overhang * 2
        y: -overhang
        x: Math.min(track.width - width, Math.max(0, track.width * (track.tick ?? 0) - width / 2))
        radius: width / 2
        color: Tokens.tick(Kirigami.Theme)

        Behavior on x {
            enabled: track.animated && track.progress >= 1

            NumberAnimation {
                duration: Kirigami.Units.longDuration
                easing.type: Easing.OutCubic
            }
        }
    }
}
