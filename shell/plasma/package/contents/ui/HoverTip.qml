import QtQuick
import org.kde.plasma.components as PlasmaComponents3

HoverHandler {
    id: hover

    property string text: ""

    property PlasmaComponents3.ToolTip tip: PlasmaComponents3.ToolTip {
        parent: hover.parent
        text: hover.text
        visible: hover.hovered && hover.text !== ""
    }
}
