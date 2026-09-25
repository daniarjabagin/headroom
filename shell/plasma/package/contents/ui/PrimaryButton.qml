import QtQuick
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami

T.Button {
    id: button

    property bool animated: true

    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    implicitHeight: implicitContentHeight + topPadding + bottomPadding
    topPadding: Math.round(Kirigami.Units.smallSpacing * 1.25)
    bottomPadding: topPadding
    leftPadding: Math.round(Kirigami.Units.gridUnit * 0.75)
    rightPadding: leftPadding
    opacity: enabled ? 1 : 0.6
    scale: button.down && button.animated ? 0.96 : 1

    contentItem: TextLabel {
        weight: Font.DemiBold
        color: Kirigami.Theme.highlightedTextColor
        text: button.text
        horizontalAlignment: Text.AlignHCenter
    }

    background: Rectangle {
        radius: height / 2
        color: button.hovered ? Qt.lighter(Kirigami.Theme.highlightColor, 1.08) : Kirigami.Theme.highlightColor
        border.width: button.visualFocus ? Kirigami.Units.smallSpacing / 2 : 0
        border.color: Kirigami.Theme.highlightedTextColor
    }

    Behavior on scale {
        enabled: button.animated

        NumberAnimation {
            duration: Kirigami.Units.shortDuration
            easing.type: Easing.OutCubic
        }
    }
}
