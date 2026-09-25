pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Tokens.js" as Tokens

Rectangle {
    id: switcher

    property var options: []
    property var current: null
    property bool animated: true
    readonly property int currentIndex: Math.max(0, options.findIndex(option => option.value === current))
    readonly property real inset: Math.round(Kirigami.Units.smallSpacing * 0.75)
    readonly property real segmentWidth: (width - inset * 2 - segments.spacing * Math.max(0, options.length - 1)) / Math.max(1, options.length)

    signal selected(var value)

    implicitWidth: options.length * Kirigami.Units.gridUnit * 4.5 + inset * 2
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
            enabled: switcher.animated

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
            model: switcher.options.length

            T.AbstractButton {
                id: segment

                required property int index
                readonly property var modelData: switcher.options[index]
                readonly property bool isCurrent: modelData.value === switcher.current

                width: switcher.segmentWidth
                implicitHeight: contentItem.implicitHeight + topPadding + bottomPadding
                topPadding: Kirigami.Units.smallSpacing
                bottomPadding: Kirigami.Units.smallSpacing
                leftPadding: Kirigami.Units.smallSpacing
                rightPadding: Kirigami.Units.smallSpacing
                hoverEnabled: true
                onClicked: switcher.selected(modelData.value)

                PointerHover {
                    id: pointer
                }

                contentItem: TextLabel {
                    role: "caption"
                    weight: segment.isCurrent ? Font.DemiBold : Font.Medium
                    emphasis: segment.isCurrent || pointer.shown ? "primary" : "secondary"
                    text: segment.modelData.label
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideRight
                }
            }
        }
    }
}
