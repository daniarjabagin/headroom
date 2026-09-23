import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Item {
    id: kcm

    default property alias pageContent: holder.data
    property string title: ""

    implicitWidth: Kirigami.Units.gridUnit * 32
    implicitHeight: holder.implicitHeight + Kirigami.Units.gridUnit * 2

    ColumnLayout {
        id: holder

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Kirigami.Units.gridUnit
    }
}
