import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/Format.js" as Format

MouseArea {
    id: compact

    property var headline: null
    property bool stale: false
    property bool showPercentage: true
    property bool vertical: false
    property bool expanded: false
    property bool wasExpanded: false
    readonly property bool hasHeadline: headline !== null
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
            visible: compact.hasHeadline
            Layout.alignment: Qt.AlignCenter
            size: compact.glyphSize - Kirigami.Units.smallSpacing / 4
            fraction: compact.hasHeadline ? compact.headline.remainingPercent / 100 : 0
            tone: compact.hasHeadline ? compact.headline.tone : "neutral"
        }

        PlasmaComponents3.Label {
            visible: compact.hasHeadline && compact.showPercentage
            Layout.alignment: Qt.AlignCenter
            text: compact.hasHeadline ? Format.panelPercent(compact.headline.remainingPercent) : ""
            font.pointSize: compact.vertical ? Kirigami.Theme.smallFont.pointSize : Kirigami.Theme.defaultFont.pointSize
            font.weight: Font.DemiBold
            font.features: {
                "tnum": 1
            }
        }
    }
}
