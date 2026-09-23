import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

Rectangle {
    id: track

    property real fraction: 0
    property string tone: "neutral"
    property var tick: null

    Layout.fillWidth: true
    implicitHeight: Metrics.meterHeight(Kirigami.Units)
    radius: height / 2
    color: Tokens.track(Kirigami.Theme)

    Rectangle {
        height: parent.height
        radius: height / 2
        width: track.fraction > 0 ? Math.max(height, track.width * track.fraction) : 0
        color: Tokens.toneColor(Kirigami.Theme, track.tone)

        Behavior on width {
            NumberAnimation {
                duration: Kirigami.Units.longDuration
                easing.type: Easing.OutCubic
            }
        }
    }

    Rectangle {
        readonly property real overhang: Metrics.tickOverhang(Kirigami.Units)

        visible: track.tick !== null
        width: overhang
        height: track.height + overhang * 2
        y: -overhang
        x: Math.min(track.width - width, Math.max(0, track.width * (track.tick ?? 0) - width / 2))
        radius: width / 2
        color: Tokens.tick(Kirigami.Theme)
    }
}
