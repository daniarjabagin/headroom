pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/PanelLayout.js" as PanelLayout
import "logic/Settings.js" as Settings

MouseArea {
    id: compact

    property var items: []
    property var panelTone: null
    property var display: Settings.parseDisplay(null)
    property string lang: "en"
    property bool stale: false
    property bool reducedMotion: false
    property bool vertical: false
    property bool expanded: false
    property bool wasExpanded: false
    readonly property real thickness: vertical ? width : height
    readonly property int glyphSize: thickness >= Kirigami.Units.iconSizes.medium ? Kirigami.Units.iconSizes.smallMedium : Kirigami.Units.iconSizes.small
    readonly property bool markShown: PanelLayout.showsMark(items, display)
    readonly property string markTone: PanelLayout.markTone(display, panelTone)
    readonly property var shownItems: markShown ? [] : PanelLayout.shownItems(items, vertical, thickness, Kirigami.Units)
    readonly property string itemKeys: PanelLayout.keys(shownItems)
    readonly property var parts: PanelLayout.parts(display, vertical)

    signal activated(bool wasExpanded)

    Layout.minimumWidth: vertical ? Kirigami.Units.iconSizes.small : grid.implicitWidth
    Layout.minimumHeight: vertical ? grid.implicitHeight : Kirigami.Units.iconSizes.small
    Layout.preferredWidth: Layout.minimumWidth
    Layout.preferredHeight: Layout.minimumHeight
    hoverEnabled: true
    opacity: !markShown && stale ? PanelLayout.staleOpacity() : 1
    onPressed: wasExpanded = expanded
    onClicked: activated(wasExpanded)

    GridLayout {
        id: grid

        anchors.centerIn: parent
        flow: compact.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
        rowSpacing: Math.round(Kirigami.Units.gridUnit * 0.72)
        columnSpacing: Math.round(Kirigami.Units.gridUnit * 0.72)

        Kirigami.Icon {
            id: mark

            objectName: "compactMark"
            visible: compact.markShown
            Layout.alignment: Qt.AlignCenter
            implicitWidth: compact.glyphSize
            implicitHeight: compact.glyphSize
            source: Qt.resolvedUrl("../icons/headroom-symbolic.svg")
            isMask: true
            color: PanelLayout.toneColor(Kirigami.Theme, compact.markTone)
        }

        Repeater {
            model: PanelLayout.keyList(compact.itemKeys)

            PanelItem {
                required property int index

                Layout.alignment: Qt.AlignCenter
                item: PanelLayout.itemAt(compact.shownItems, index)
                parts: compact.parts
                vertical: compact.vertical
                glyphSize: compact.glyphSize
                lang: compact.lang
                valueMode: compact.display.valueMode
                reducedMotion: compact.reducedMotion
            }
        }
    }

    PanelPulse {
        subject: mark
        running: compact.markShown && PanelLayout.pulses(compact.markTone, Kirigami.Units, compact.reducedMotion)
    }
}
