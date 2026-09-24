import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/Tokens.js" as Tokens

T.Button {
    id: button

    property bool busy: false
    property bool animated: true

    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    implicitHeight: implicitContentHeight + topPadding + bottomPadding
    topPadding: Math.round(Kirigami.Units.smallSpacing * 0.75)
    bottomPadding: topPadding
    leftPadding: Kirigami.Units.largeSpacing
    rightPadding: Kirigami.Units.largeSpacing
    enabled: !busy
    opacity: enabled || busy ? 1 : 0.6
    scale: button.down ? 0.96 : 1

    contentItem: RowLayout {
        spacing: Kirigami.Units.smallSpacing

        SpinningIcon {
            objectName: "smallButtonSpinner"
            visible: button.busy
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Metrics.tinyIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: "view-refresh"
            isMask: true
            color: Tokens.secondaryText(Kirigami.Theme)
            spinning: button.busy
            animated: button.animated
        }

        TextLabel {
            Layout.alignment: Qt.AlignVCenter
            role: "caption"
            weight: Font.DemiBold
            emphasis: button.busy ? "secondary" : "primary"
            text: button.text
            horizontalAlignment: Text.AlignHCenter
        }
    }

    background: Rectangle {
        radius: Metrics.buttonRadius(Kirigami.Units)
        color: pointer.shown ? Tokens.chip(Kirigami.Theme) : Tokens.control(Kirigami.Theme)
        border.width: Metrics.hairline(Kirigami.Units)
        border.color: button.visualFocus ? Kirigami.Theme.highlightColor : Tokens.separator(Kirigami.Theme)

        Behavior on color {
            ColorAnimation {
                duration: Motion.hoverDuration(Kirigami.Units)
                easing.type: Easing.OutCubic
            }
        }
    }

    PointerHover {
        id: pointer

        enabled: button.enabled
    }

    Behavior on scale {
        NumberAnimation {
            duration: Kirigami.Units.shortDuration
            easing.type: Easing.OutCubic
        }
    }
}
