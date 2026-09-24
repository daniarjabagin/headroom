import QtQuick

PointerHover {
    id: hover

    property string text: ""

    property PlainToolTip tip: PlainToolTip {
        parent: hover.parent
        text: hover.text
        visible: hover.shown && hover.text !== ""
    }
}
