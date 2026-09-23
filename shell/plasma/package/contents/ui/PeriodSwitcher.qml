pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Spend.js" as Spend
import "logic/Tokens.js" as Tokens

Rectangle {
    id: switcher

    property string current: "today"
    readonly property int currentIndex: Math.max(0, Spend.PERIODS.findIndex(period => period.key === current))
    readonly property real inset: Math.round(Kirigami.Units.smallSpacing * 0.75)
    readonly property real segmentWidth: (width - inset * 2 - segments.spacing * (Spend.PERIODS.length - 1)) / Spend.PERIODS.length

    signal selected(string key)

    Layout.fillWidth: true
    implicitHeight: segments.implicitHeight + inset * 2
    radius: height / 2
    color: Tokens.chip(Kirigami.Theme)

    Kirigami.ShadowedRectangle {
        x: switcher.inset + switcher.currentIndex * (switcher.segmentWidth + segments.spacing)
        y: switcher.inset
        width: switcher.segmentWidth
        height: segments.height
        radius: height / 2
        color: Kirigami.Theme.backgroundColor
        shadow.size: Kirigami.Units.smallSpacing / 2
        shadow.yOffset: Kirigami.Units.smallSpacing / 4
        shadow.color: Tokens.controlShadow()

        Behavior on x {
            NumberAnimation {
                duration: Kirigami.Units.longDuration
                easing.type: Easing.OutCubic
            }
        }
    }

    Row {
        id: segments

        x: switcher.inset
        y: switcher.inset
        spacing: Kirigami.Units.smallSpacing / 2

        Repeater {
            model: Spend.PERIODS

            T.AbstractButton {
                id: segment

                required property var modelData
                readonly property bool isCurrent: modelData.key === switcher.current

                width: switcher.segmentWidth
                implicitHeight: contentItem.implicitHeight + topPadding + bottomPadding
                topPadding: Kirigami.Units.smallSpacing
                bottomPadding: Kirigami.Units.smallSpacing
                onClicked: switcher.selected(modelData.key)

                contentItem: TextLabel {
                    role: "caption"
                    weight: segment.isCurrent ? Font.DemiBold : Font.Medium
                    emphasis: segment.isCurrent || segment.hovered ? "primary" : "secondary"
                    text: segment.modelData.title
                    horizontalAlignment: Text.AlignHCenter
                }
            }
        }
    }
}
