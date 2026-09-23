import QtQuick
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import headroom.preview

T.ToolBar {
    id: heading

    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    implicitHeight: implicitContentHeight + topPadding + bottomPadding
    topPadding: PreviewConfig.dialogPadding
    leftInset: -PreviewConfig.dialogPadding
    rightInset: -PreviewConfig.dialogPadding
    bottomInset: -PreviewConfig.dialogPadding

    background: Rectangle {
        color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g, Kirigami.Theme.textColor.b, 0.04)
        bottomLeftRadius: PreviewConfig.dialogRadius
        bottomRightRadius: PreviewConfig.dialogRadius

        Rectangle {
            width: parent.width
            height: 1
            color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g, Kirigami.Theme.textColor.b, 0.12)
        }
    }
}
