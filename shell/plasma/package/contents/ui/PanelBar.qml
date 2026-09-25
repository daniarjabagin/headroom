import QtQuick
import org.kde.kirigami as Kirigami
import "logic/Motion.js" as Motion
import "logic/PanelLayout.js" as PanelLayout
import "logic/Tokens.js" as Tokens

Item {
    id: bar

    property real fraction: 0
    property var tick: null
    property string tone: "neutral"
    property bool reducedMotion: false
    property real shownFraction: fraction
    readonly property int barHeight: Math.round(Kirigami.Units.smallSpacing * 1.25)
    readonly property int tickWidth: Math.round(Kirigami.Units.smallSpacing / 2)

    implicitWidth: Math.round(Kirigami.Units.gridUnit * 1.45)
    implicitHeight: barHeight + tickWidth * 2

    Rectangle {
        id: track

        anchors.verticalCenter: parent.verticalCenter
        width: parent.width
        height: bar.barHeight
        radius: height / 2
        color: Tokens.panelTrack(Kirigami.Theme)

        Rectangle {
            height: parent.height
            radius: height / 2
            width: bar.shownFraction > 0 ? Math.max(height, parent.width * bar.shownFraction) : 0
            color: PanelLayout.toneColor(Kirigami.Theme, bar.tone)
        }
    }

    Rectangle {
        visible: bar.tick !== null
        x: Math.round(Math.min(bar.width - width, Math.max(0, bar.width * (bar.tick ?? 0) - width / 2)))
        width: bar.tickWidth
        height: bar.height
        radius: width / 2
        color: PanelLayout.tickColor(Kirigami.Theme)
    }

    Behavior on shownFraction {
        NumberAnimation {
            duration: Motion.enabled(Kirigami.Units, bar.reducedMotion) ? Kirigami.Units.longDuration : 0
            easing.type: Easing.OutCubic
        }
    }
}
