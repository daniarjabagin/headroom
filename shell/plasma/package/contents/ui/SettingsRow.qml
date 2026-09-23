import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

Item {
    id: settingsRow

    property string title: ""
    property string subtitle: ""
    property bool separated: true
    default property alias controls: trailing.data

    Layout.fillWidth: true
    implicitHeight: row.implicitHeight + Kirigami.Units.largeSpacing * 2

    Rectangle {
        visible: settingsRow.separated
        x: Metrics.rowInset(Kirigami.Units)
        width: parent.width - x * 2
        height: Metrics.hairline(Kirigami.Units)
        color: Tokens.separator(Kirigami.Theme)
    }

    RowLayout {
        id: row

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: Metrics.rowInset(Kirigami.Units)
        anchors.rightMargin: Metrics.rowInset(Kirigami.Units)
        spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Math.round(Kirigami.Units.smallSpacing / 2)

            TextLabel {
                Layout.fillWidth: true
                role: "label"
                weight: Font.Medium
                text: settingsRow.title
                wrapMode: Text.Wrap
            }

            TextLabel {
                visible: settingsRow.subtitle !== ""
                Layout.fillWidth: true
                role: "caption"
                emphasis: "secondary"
                text: settingsRow.subtitle
                wrapMode: Text.Wrap
            }
        }

        RowLayout {
            id: trailing

            Layout.alignment: Qt.AlignVCenter
            spacing: Kirigami.Units.smallSpacing
        }
    }
}
