pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import headroom.preview
import "../../package/contents/ui" as Ui
import "../../package/contents/ui/logic/Settings.js" as Settings

ColumnLayout {
    id: samples

    readonly property var headlines: [sample("codex", "session", 62, "good"), sample("codex", "weekly", 29, "warning"), sample("claude", "session", 17, "critical"), null]
    readonly property var windowDisplay: Object.assign({}, Settings.parseDisplay(null), {
        panelLabel: "window"
    })

    function sample(provider, windowId, remainingPercent, tone) {
        return {
            provider,
            windowId,
            windowLabel: windowId,
            usedPercent: 100 - remainingPercent,
            remainingPercent,
            tone
        };
    }

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

    Repeater {
        model: samples.headlines.length - 1

        Rectangle {
            id: windowStrip

            required property int index

            implicitWidth: windowSample.Layout.minimumWidth + Kirigami.Units.largeSpacing * 4
            implicitHeight: 44
            radius: PreviewConfig.dialogRadius
            color: Kirigami.Theme.backgroundColor

            Ui.CompactRepresentation {
                id: windowSample

                anchors.centerIn: parent
                width: Layout.minimumWidth
                height: parent.height
                headline: samples.headlines[windowStrip.index]
                display: samples.windowDisplay
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
