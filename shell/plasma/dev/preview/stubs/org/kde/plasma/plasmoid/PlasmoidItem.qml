import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import headroom.preview

Item {
    id: plasmoidItem

    property Component compactRepresentation
    property Component fullRepresentation
    property bool expanded: true
    property string toolTipMainText: ""
    property string toolTipSubText: ""
    readonly property int panelThickness: 44
    readonly property Item fullItem: fullLoader.item as Item
    readonly property real fullWidth: fullItem ? fullItem.Layout.preferredWidth : 0
    readonly property real fullHeight: fullItem ? fullItem.Layout.preferredHeight : 0

    implicitWidth: popup.width
    implicitHeight: panel.height + PreviewConfig.dialogPadding + popup.height

    Rectangle {
        id: panel

        width: popup.width
        height: plasmoidItem.panelThickness
        radius: PreviewConfig.dialogRadius
        color: Kirigami.Theme.backgroundColor

        Loader {
            anchors.right: parent.right
            anchors.rightMargin: Kirigami.Units.largeSpacing
            anchors.verticalCenter: parent.verticalCenter
            height: parent.height
            width: (item as Item)?.Layout.preferredWidth ?? 0
            sourceComponent: plasmoidItem.compactRepresentation
        }
    }

    Rectangle {
        id: popup

        y: panel.height + PreviewConfig.dialogPadding
        width: plasmoidItem.fullWidth + PreviewConfig.dialogPadding * 2
        height: plasmoidItem.fullHeight + PreviewConfig.dialogPadding * 2
        radius: PreviewConfig.dialogRadius
        color: Kirigami.Theme.backgroundColor
        border.width: 1
        border.color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g, Kirigami.Theme.textColor.b, 0.15)

        Loader {
            id: fullLoader

            x: PreviewConfig.dialogPadding
            y: PreviewConfig.dialogPadding
            width: plasmoidItem.fullWidth
            height: plasmoidItem.fullHeight
            sourceComponent: plasmoidItem.fullRepresentation
        }
    }
}
