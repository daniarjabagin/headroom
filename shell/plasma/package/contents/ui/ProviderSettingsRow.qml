import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

Item {
    id: providerRow

    property string provider: ""
    property string title: ""
    property string subtitle: ""
    default property alias controls: trailing.data
    property alias leading: lead.data

    Layout.fillWidth: true
    implicitHeight: row.implicitHeight + Kirigami.Units.largeSpacing * 2

    Rectangle {
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
        spacing: Kirigami.Units.largeSpacing

        RowLayout {
            id: lead

            visible: children.length > 0
            spacing: 0
        }

        ProviderIcon {
            provider: providerRow.provider
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Kirigami.Units.iconSizes.smallMedium
            implicitHeight: implicitWidth
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Math.round(Kirigami.Units.smallSpacing / 2)

            TextLabel {
                Layout.fillWidth: true
                role: "label"
                weight: Font.Medium
                text: providerRow.title
                elide: Text.ElideRight
            }

            TextLabel {
                visible: providerRow.subtitle !== ""
                Layout.fillWidth: true
                role: "caption"
                emphasis: "secondary"
                text: providerRow.subtitle
                elide: Text.ElideRight
            }
        }

        RowLayout {
            id: trailing

            Layout.alignment: Qt.AlignVCenter
            spacing: Kirigami.Units.smallSpacing
        }
    }
}
