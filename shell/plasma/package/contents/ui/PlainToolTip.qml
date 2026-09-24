import QtQuick
import org.kde.plasma.components as PlasmaComponents3

PlasmaComponents3.ToolTip {
    id: tip

    contentItem: PlasmaComponents3.Label {
        objectName: "plainToolTipLabel"
        text: tip.text
        textFormat: Text.PlainText
        wrapMode: Text.WordWrap
    }
}
