import QtQuick
import QtQuick.Controls.Basic as Basic
import org.kde.kirigami as Kirigami

Basic.ToolTip {
    id: tip

    padding: Kirigami.Units.largeSpacing
    x: parent ? parent.width - width : 0
    y: parent ? parent.height + Kirigami.Units.smallSpacing : 0

    background: Rectangle {
        radius: Kirigami.Units.smallSpacing * 1.5
        color: Kirigami.Theme.backgroundColor
        border.width: 1
        border.color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g, Kirigami.Theme.textColor.b, 0.15)
    }
}
