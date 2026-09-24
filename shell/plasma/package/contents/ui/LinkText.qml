import QtQuick
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami

T.AbstractButton {
    id: link

    implicitWidth: label.implicitWidth
    implicitHeight: label.implicitHeight
    hoverEnabled: true
    Accessible.role: Accessible.Link
    Accessible.name: text

    contentItem: TextLabel {
        id: label

        role: "caption"
        color: Kirigami.Theme.highlightColor
        font.underline: pointer.shown || link.visualFocus
        text: link.text
    }

    PointerHover {
        id: pointer
    }
}
