import QtQuick
import org.kde.plasma.components as PlasmaComponents3

PointerHover {
    id: hover

    property string text: ""

    property PlasmaComponents3.ToolTip tip: PlasmaComponents3.ToolTip {
        parent: hover.parent
        text: hover.text
        visible: hover.shown && hover.text !== ""
    }
}
