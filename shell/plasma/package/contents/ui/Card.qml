import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/Tokens.js" as Tokens

Rectangle {
    id: card

    default property alias content: column.data
    property real verticalPadding: Metrics.gutter(Kirigami.Units)
    property alias spacing: column.spacing
    property bool hoverable: false
    property bool lifted: false
    readonly property bool tinted: lifted || (hoverable && hover.shown)

    Layout.fillWidth: true
    implicitHeight: column.implicitHeight + verticalPadding * 2
    radius: Metrics.cardRadius(Kirigami.Units)
    color: tinted ? Tokens.cardHover(Kirigami.Theme) : Tokens.card(Kirigami.Theme)
    border.width: lifted ? Metrics.hairline(Kirigami.Units) : 0
    border.color: Tokens.separator(Kirigami.Theme)

    PointerHover {
        id: hover

        enabled: card.hoverable
    }

    ColumnLayout {
        id: column

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.topMargin: card.verticalPadding
        spacing: 0
    }

    Behavior on color {
        ColorAnimation {
            duration: Motion.hoverDuration(Kirigami.Units)
            easing.type: Easing.OutCubic
        }
    }
}
