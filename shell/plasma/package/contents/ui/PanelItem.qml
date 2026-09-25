import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/Format.js" as Format
import "logic/Motion.js" as Motion
import "logic/PanelLayout.js" as PanelLayout

Item {
    id: panelItem

    required property var item
    required property var parts
    property bool vertical: false
    property int glyphSize: Kirigami.Units.iconSizes.small
    property string lang: "en"
    property string valueMode: "left"
    property bool reducedMotion: false
    readonly property string tone: item.tone
    readonly property real valuePointSize: vertical ? Kirigami.Theme.smallFont.pointSize : Kirigami.Theme.defaultFont.pointSize

    implicitWidth: grid.implicitWidth
    implicitHeight: grid.implicitHeight
    opacity: 0
    Component.onCompleted: entry.start()

    GridLayout {
        id: grid

        anchors.centerIn: parent
        flow: panelItem.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
        rowSpacing: Math.round(Kirigami.Units.smallSpacing * 0.75)
        columnSpacing: Math.round(Kirigami.Units.smallSpacing * 1.25)

        ProviderIcon {
            visible: panelItem.parts.logo
            Layout.alignment: Qt.AlignCenter
            implicitWidth: panelItem.glyphSize
            implicitHeight: panelItem.glyphSize
            provider: panelItem.item.logo
            color: Kirigami.Theme.textColor
        }

        PlasmaComponents3.Label {
            objectName: "compactCountLabel"
            visible: panelItem.parts.letter && PanelLayout.showsCount(panelItem.item)
            Layout.alignment: Qt.AlignCenter
            opacity: 0.67
            textFormat: Text.PlainText
            text: PanelLayout.showsCount(panelItem.item) ? Format.panelCount(panelItem.item.accountCount) : ""
            font.pointSize: Kirigami.Theme.smallFont.pointSize
            font.weight: Font.ExtraBold
        }

        PlasmaComponents3.Label {
            objectName: "compactWindowLabel"
            visible: panelItem.parts.letter
            Layout.alignment: Qt.AlignCenter
            opacity: 0.67
            textFormat: Text.PlainText
            text: panelItem.parts.letter ? Format.shortWindowLabel(panelItem.lang, panelItem.item.windowId, panelItem.item.windowLabel) : ""
            font.pointSize: Kirigami.Theme.smallFont.pointSize
            font.weight: Font.ExtraBold
        }

        PanelRing {
            visible: panelItem.parts.indicator === "ring"
            Layout.alignment: Qt.AlignCenter
            size: panelItem.glyphSize - Kirigami.Units.smallSpacing / 4
            fraction: PanelLayout.fraction(panelItem.item)
            tone: panelItem.tone
            reducedMotion: panelItem.reducedMotion
        }

        PanelBar {
            visible: panelItem.parts.indicator === "bar"
            Layout.alignment: Qt.AlignCenter
            fraction: PanelLayout.fraction(panelItem.item)
            tick: PanelLayout.tickPosition(panelItem.item, panelItem.valueMode)
            tone: panelItem.tone
            reducedMotion: panelItem.reducedMotion
        }

        PlasmaComponents3.Label {
            objectName: "compactPercentLabel"
            visible: panelItem.parts.value
            Layout.alignment: Qt.AlignCenter
            textFormat: Text.PlainText
            text: PanelLayout.valueText(panelItem.item)
            color: PanelLayout.valueColor(Kirigami.Theme, panelItem.tone, panelItem.parts.tintedValue)
            font.pointSize: panelItem.valuePointSize
            font.weight: Font.DemiBold
            font.features: {
                "tnum": 1
            }
        }
    }

    PanelPulse {
        subject: grid
        running: PanelLayout.pulses(panelItem.tone, Kirigami.Units, panelItem.reducedMotion)
    }

    NumberAnimation {
        id: entry

        target: panelItem
        property: "opacity"
        to: 1
        duration: Motion.enabled(Kirigami.Units, panelItem.reducedMotion) ? Kirigami.Units.longDuration : 0
        easing.type: Easing.OutCubic
    }
}
