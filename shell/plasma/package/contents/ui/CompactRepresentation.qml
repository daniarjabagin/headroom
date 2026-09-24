import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/Format.js" as Format
import "logic/Settings.js" as Settings
import "logic/State.js" as State

MouseArea {
    id: compact

    property var headline: null
    property var display: Settings.parseDisplay(null)
    property string lang: "en"
    property bool stale: false
    property bool reducedMotion: false
    property bool vertical: false
    property bool expanded: false
    property bool wasExpanded: false
    readonly property bool hasHeadline: headline !== null
    readonly property bool windowMode: hasHeadline && display.panelLabel === "window"
    readonly property real percent: hasHeadline ? State.headlinePercent(headline, display.valueMode) : 0
    readonly property real thickness: vertical ? width : height
    readonly property int glyphSize: thickness >= Kirigami.Units.iconSizes.medium ? Kirigami.Units.iconSizes.smallMedium : Kirigami.Units.iconSizes.small

    signal activated(bool wasExpanded)

    Layout.minimumWidth: vertical ? Kirigami.Units.iconSizes.small : grid.implicitWidth
    Layout.minimumHeight: vertical ? grid.implicitHeight : Kirigami.Units.iconSizes.small
    Layout.preferredWidth: Layout.minimumWidth
    Layout.preferredHeight: Layout.minimumHeight
    hoverEnabled: true
    opacity: hasHeadline && stale ? 0.55 : 1
    onPressed: wasExpanded = expanded
    onClicked: activated(wasExpanded)

    GridLayout {
        id: grid

        anchors.centerIn: parent
        flow: compact.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
        rowSpacing: 0
        columnSpacing: Math.round(Kirigami.Units.smallSpacing * 1.25)

        Kirigami.Icon {
            visible: !compact.hasHeadline
            Layout.alignment: Qt.AlignCenter
            implicitWidth: compact.glyphSize
            implicitHeight: compact.glyphSize
            source: Qt.resolvedUrl("../icons/headroom-symbolic.svg")
            isMask: true
            color: Kirigami.Theme.textColor
        }

        PanelRing {
            visible: compact.hasHeadline && !compact.windowMode
            Layout.alignment: Qt.AlignCenter
            size: compact.glyphSize - Kirigami.Units.smallSpacing / 4
            fraction: compact.percent / 100
            tone: compact.hasHeadline ? compact.headline.tone : "neutral"
            reducedMotion: compact.reducedMotion
        }

        ProviderIcon {
            visible: compact.windowMode
            Layout.alignment: Qt.AlignCenter
            implicitWidth: compact.glyphSize
            implicitHeight: compact.glyphSize
            provider: compact.headline?.provider ?? "unknown"
            color: Kirigami.Theme.textColor
        }

        PlasmaComponents3.Label {
            objectName: "compactWindowLabel"
            visible: compact.windowMode
            Layout.alignment: Qt.AlignCenter
            opacity: 0.7
            textFormat: Text.PlainText
            text: compact.windowMode ? Format.shortWindowLabel(compact.lang, compact.headline.windowId, compact.headline.windowLabel) : ""
            font.pointSize: compact.vertical ? Kirigami.Theme.smallFont.pointSize : Kirigami.Theme.defaultFont.pointSize
            font.weight: Font.DemiBold
        }

        PlasmaComponents3.Label {
            objectName: "compactPercentLabel"
            visible: compact.hasHeadline
            Layout.alignment: Qt.AlignCenter
            textFormat: Text.PlainText
            text: compact.hasHeadline ? Format.panelPercent(compact.percent) : ""
            color: compact.windowMode && compact.headline.tone === "critical" ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor
            font.pointSize: compact.vertical ? Kirigami.Theme.smallFont.pointSize : Kirigami.Theme.defaultFont.pointSize
            font.weight: Font.DemiBold
            font.features: {
                "tnum": 1
            }
        }
    }
}
