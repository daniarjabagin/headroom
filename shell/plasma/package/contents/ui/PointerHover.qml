import QtQuick
import "logic/Motion.js" as Motion

HoverHandler {
    id: pointer

    readonly property bool windowActive: (parent as Item)?.Window.active ?? false
    property bool armed: false
    property point restingAt: Qt.point(0, 0)
    readonly property bool shown: hovered && armed

    onWindowActiveChanged: {
        if (!windowActive)
            armed = false;
    }
    onHoveredChanged: {
        if (hovered && !armed)
            restingAt = point.scenePosition;
    }
    onPointChanged: {
        if (hovered && !armed && Motion.moved(restingAt, point.scenePosition))
            armed = true;
    }
}
