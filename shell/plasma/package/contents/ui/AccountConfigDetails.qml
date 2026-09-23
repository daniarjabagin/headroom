pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Format.js" as Format
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Providers.js" as Providers
import "logic/Settings.js" as Settings

Item {
    id: details

    required property var account
    required property var display
    required property string lang
    required property bool expanded
    property bool confirming: false

    signal labelApplied(string label)
    signal windowToggled(string windowId, bool hidden)
    signal removeConfirmed

    function tr(msgid, values) {
        return I18n.tr(lang, msgid, values);
    }

    Layout.fillWidth: true
    Layout.preferredHeight: expanded ? rows.implicitHeight + Kirigami.Units.largeSpacing : 0
    visible: expanded || heightAnimation.running
    clip: true
    opacity: expanded ? 1 : 0
    onExpandedChanged: confirming = false

    ColumnLayout {
        id: rows

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: Kirigami.Units.gridUnit * 2 + Kirigami.Units.largeSpacing
        anchors.rightMargin: Metrics.rowInset(Kirigami.Units)
        spacing: Kirigami.Units.mediumSpacing

        RowLayout {
            spacing: Kirigami.Units.smallSpacing

            QQC2.TextField {
                id: labelField

                Layout.fillWidth: true
                text: details.account.label ?? ""
                placeholderText: details.tr("Label")
                maximumLength: 64
                onAccepted: details.labelApplied(text.trim())
            }

            SmallButton {
                text: details.tr("Apply")
                enabled: labelField.text.trim() !== (details.account.label ?? "")
                onClicked: details.labelApplied(labelField.text.trim())
            }
        }

        TextLabel {
            visible: details.account.windows.length > 0
            Layout.topMargin: Kirigami.Units.smallSpacing
            role: "caption"
            weight: Font.DemiBold
            emphasis: "secondary"
            text: details.tr("Limits")
        }

        Repeater {
            model: details.account.windows

            RowLayout {
                id: windowRow

                required property var modelData

                spacing: Kirigami.Units.largeSpacing

                TextLabel {
                    Layout.fillWidth: true
                    text: Format.windowLabel(details.lang, windowRow.modelData)
                    elide: Text.ElideRight
                }

                TextLabel {
                    emphasis: "secondary"
                    text: windowRow.modelData.remainingPercent === null ? details.tr("No data yet") : Format.percentLeft(details.lang, windowRow.modelData.remainingPercent)
                }

                QQC2.Switch {
                    checked: !Settings.isWindowHidden(details.display, details.account.id, windowRow.modelData.id)
                    onToggled: details.windowToggled(windowRow.modelData.id, !checked)
                }
            }
        }

        RowLayout {
            visible: details.account.owner === "headroom" && !details.confirming
            Layout.topMargin: Kirigami.Units.smallSpacing
            spacing: Kirigami.Units.largeSpacing

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                TextLabel {
                    text: details.tr("Remove from Headroom")
                }

                TextLabel {
                    Layout.fillWidth: true
                    role: "caption"
                    emphasis: "secondary"
                    wrapMode: Text.Wrap
                    text: details.tr("Deletes the sign-in Headroom created for this account")
                }
            }

            SmallButton {
                text: details.tr("Remove")
                onClicked: details.confirming = true
            }
        }

        NoticeRow {
            visible: details.confirming
            Layout.leftMargin: 0
            Layout.rightMargin: 0
            entry: ({
                    kind: "error",
                    title: details.tr("Remove {name}?", {
                        name: Providers.accountName(details.account)
                    }),
                    detail: details.tr("Headroom deletes the sign-in it created for this account. The account itself is not affected."),
                    note: "",
                    actions: [
                        {
                            kind: "cancel",
                            label: details.tr("Cancel"),
                            value: ""
                        },
                        {
                            kind: "remove",
                            label: details.tr("Remove"),
                            value: details.account.id
                        }
                    ]
                })
            onActionTriggered: kind => {
                details.confirming = false;
                if (kind === "remove")
                    details.removeConfirmed();
            }
        }
    }

    Behavior on Layout.preferredHeight {
        NumberAnimation {
            id: heightAnimation

            duration: Kirigami.Units.longDuration
            easing.type: Easing.OutCubic
        }
    }

    Behavior on opacity {
        NumberAnimation {
            duration: Kirigami.Units.longDuration
        }
    }
}
