import QtQuick

Item {
    id: shadowed

    property real radius: 0
    property color color: "white"
    readonly property ShadowGroup shadow: ShadowGroup {}

    Rectangle {
        x: shadowed.shadow.xOffset
        y: shadowed.shadow.yOffset
        width: parent.width
        height: parent.height
        radius: shadowed.radius
        color: shadowed.shadow.color
        visible: shadowed.shadow.size > 0
    }

    Rectangle {
        anchors.fill: parent
        radius: shadowed.radius
        color: shadowed.color
    }
}
