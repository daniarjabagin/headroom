import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

Rectangle {
    id: card

    default property alias content: column.data
    property real verticalPadding: Metrics.gutter(Kirigami.Units)
    property alias spacing: column.spacing

    Layout.fillWidth: true
    implicitHeight: column.implicitHeight + verticalPadding * 2
    radius: Metrics.cardRadius(Kirigami.Units)
    color: Tokens.card(Kirigami.Theme)

    ColumnLayout {
        id: column

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.topMargin: card.verticalPadding
        spacing: 0
    }
}
