pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import headroom.preview
import "../../package/contents/ui" as Ui

ColumnLayout {
    id: samples

    readonly property var headlines: [
        {
            remainingPercent: 62,
            tone: "good"
        },
        {
            remainingPercent: 29,
            tone: "warning"
        },
        {
            remainingPercent: 17,
            tone: "critical"
        },
        null]

    spacing: Kirigami.Units.largeSpacing

    Repeater {
        model: samples.headlines.length

        Rectangle {
            id: strip

            required property int index

            implicitWidth: sample.Layout.minimumWidth + Kirigami.Units.largeSpacing * 4
            implicitHeight: 44
            radius: PreviewConfig.dialogRadius
            color: Kirigami.Theme.backgroundColor

            Ui.CompactRepresentation {
                id: sample

                anchors.centerIn: parent
                width: Layout.minimumWidth
                height: parent.height
                headline: samples.headlines[strip.index]
            }
        }
    }

    Rectangle {
        implicitWidth: 44
        implicitHeight: vertical.Layout.minimumHeight + Kirigami.Units.largeSpacing * 4
        radius: PreviewConfig.dialogRadius
        color: Kirigami.Theme.backgroundColor

        Ui.CompactRepresentation {
            id: vertical

            anchors.centerIn: parent
            width: parent.width
            height: Layout.minimumHeight
            vertical: true
            headline: samples.headlines[0]
        }
    }
}
