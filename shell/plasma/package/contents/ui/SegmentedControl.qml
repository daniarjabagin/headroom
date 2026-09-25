pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
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
    readonly property real gaps: segments.spacing * Math.max(0, options.length - 1)
    readonly property real widest: Math.max(0, ...Array.from(segments.children).map(child => child.naturalWidth ?? 0))
    readonly property real naturalWidth: widest * options.length + gaps + inset * 2
    readonly property real segmentWidth: Math.max(widest, (width - inset * 2 - gaps) / Math.max(1, options.length))

    signal selected(var value)

    implicitWidth: Math.max(options.length * Kirigami.Units.gridUnit * 4.5 + inset * 2, naturalWidth)
    Layout.minimumWidth: naturalWidth
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
                readonly property real naturalWidth: Math.ceil(metrics.advanceWidth) + leftPadding + rightPadding

                width: switcher.segmentWidth
                implicitHeight: contentItem.implicitHeight + topPadding + bottomPadding
                topPadding: Kirigami.Units.smallSpacing
                bottomPadding: Kirigami.Units.smallSpacing
                leftPadding: Kirigami.Units.mediumSpacing
                rightPadding: Kirigami.Units.mediumSpacing
                hoverEnabled: true
                onClicked: switcher.selected(modelData.value)

                PointerHover {
                    id: pointer
                }

                TextMetrics {
                    id: metrics

                    text: segment.modelData.label
                    font.family: label.font.family
                    font.pointSize: label.font.pointSize
                    font.weight: Font.DemiBold
                }

                contentItem: TextLabel {
                    id: label

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
