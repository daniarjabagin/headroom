pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/Tokens.js" as Tokens

Item {
    id: picker

    required property var providers
    required property string selected
    readonly property real selectedFill: 0.14

    signal picked(string providerId)

    Layout.fillWidth: true
    implicitHeight: flow.implicitHeight + Kirigami.Units.largeSpacing * 2

    Flow {
        id: flow

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: Metrics.rowInset(Kirigami.Units)
        anchors.rightMargin: Metrics.rowInset(Kirigami.Units)
        spacing: Kirigami.Units.smallSpacing

        Repeater {
            model: picker.providers

            ProviderChip {}
        }
    }

    component ProviderChip: T.AbstractButton {
        id: chip

        required property var modelData
        readonly property bool current: modelData.id === picker.selected

        text: modelData.name
        hoverEnabled: true
        topPadding: Math.round(Kirigami.Units.smallSpacing * 0.75)
        bottomPadding: topPadding
        leftPadding: Kirigami.Units.mediumSpacing
        rightPadding: Kirigami.Units.largeSpacing
        implicitWidth: implicitContentWidth + leftPadding + rightPadding
        implicitHeight: implicitContentHeight + topPadding + bottomPadding
        Accessible.role: Accessible.RadioButton
        Accessible.checked: current
        onClicked: picker.picked(modelData.id)

        PointerHover {
            id: pointer
        }

        contentItem: RowLayout {
            spacing: Kirigami.Units.smallSpacing

            ProviderIcon {
                Layout.alignment: Qt.AlignVCenter
                provider: chip.modelData.id
            }

            TextLabel {
                Layout.alignment: Qt.AlignVCenter
                role: "caption"
                weight: chip.current ? Font.DemiBold : Font.Medium
                text: chip.text
            }
        }

        background: Rectangle {
            radius: Metrics.buttonRadius(Kirigami.Units)
            color: chip.current ? Tokens.alpha(Kirigami.Theme.highlightColor, picker.selectedFill) : (pointer.shown ? Tokens.chip(Kirigami.Theme) : Tokens.control(Kirigami.Theme))
            border.width: Metrics.hairline(Kirigami.Units)
            border.color: chip.current || chip.visualFocus ? Kirigami.Theme.highlightColor : Tokens.separator(Kirigami.Theme)

            Behavior on color {
                ColorAnimation {
                    duration: Motion.hoverDuration(Kirigami.Units)
                    easing.type: Easing.OutCubic
                }
            }
        }
    }
}
