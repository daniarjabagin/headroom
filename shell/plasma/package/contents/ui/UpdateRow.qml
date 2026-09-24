import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens
import "logic/Update.js" as Update

Rectangle {
    id: updateRow

    required property UpdateActions updater
    required property string lang
    required property bool animated
    readonly property var update: updater.update
    readonly property string kind: updater.kind
    readonly property var run: kind === "install" ? updater.run : Update.IDLE
    readonly property string line: Update.runLine(lang, run)
    property bool expanded: false

    function activate() {
        if (kind === "command")
            expanded = !expanded;
        else
            updater.trigger();
    }

    visible: update !== null
    implicitHeight: content.implicitHeight + content.anchors.topMargin * 2
    color: Tokens.updateFill(Kirigami.Theme)

    Rectangle {
        width: parent.width
        height: Metrics.hairline(Kirigami.Units)
        color: Tokens.separator(Kirigami.Theme)
    }

    ColumnLayout {
        id: content

        anchors.fill: parent
        anchors.topMargin: Kirigami.Units.largeSpacing + Math.round(Kirigami.Units.smallSpacing / 2)
        anchors.bottomMargin: anchors.topMargin
        anchors.leftMargin: Metrics.rowInset(Kirigami.Units)
        anchors.rightMargin: Metrics.rowInset(Kirigami.Units)
        spacing: Kirigami.Units.largeSpacing

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing + Math.round(Kirigami.Units.smallSpacing / 2)

            Kirigami.Icon {
                Layout.alignment: Qt.AlignVCenter
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
                source: "system-software-update"
                isMask: true
                color: Kirigami.Theme.highlightColor
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: 0

                TextLabel {
                    objectName: "updateTitle"
                    Layout.fillWidth: true
                    role: "caption"
                    weight: Font.DemiBold
                    wrapMode: Text.Wrap
                    text: updateRow.update !== null ? Update.title(updateRow.lang, updateRow.update) : ""
                }

                UpdateStatus {
                    visible: updateRow.line !== ""
                    Layout.fillWidth: true
                    run: updateRow.run
                    line: updateRow.line
                    animated: updateRow.animated
                }

                LinkText {
                    objectName: "updateWhatsNew"
                    visible: updateRow.line === "" && updateRow.update !== null && Update.showsWhatsNew(updateRow.update)
                    text: I18n.tr(updateRow.lang, "What's new")
                    onClicked: updateRow.updater.openRelease()
                }
            }

            SmallButton {
                objectName: "updateAction"
                visible: updateRow.kind !== "" && (updateRow.run.phase === "idle" || updateRow.run.phase === "failed")
                Layout.alignment: Qt.AlignVCenter
                primary: updateRow.kind === "install" && updateRow.run.phase === "idle"
                animated: updateRow.animated
                text: updateRow.run.phase === "failed" ? I18n.tr(updateRow.lang, "Retry") : Update.actionLabel(updateRow.lang, updateRow.kind)
                onClicked: updateRow.activate()
            }
        }

        RowLayout {
            objectName: "updateCommand"
            visible: updateRow.kind === "command" && updateRow.expanded
            Layout.fillWidth: true
            Layout.leftMargin: Kirigami.Units.iconSizes.small + Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.mediumSpacing

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: commandText.implicitHeight + Kirigami.Units.smallSpacing * 2
                radius: Metrics.buttonRadius(Kirigami.Units)
                color: Tokens.chip(Kirigami.Theme)

                TextLabel {
                    id: commandText

                    anchors.fill: parent
                    anchors.margins: Kirigami.Units.smallSpacing
                    anchors.leftMargin: Kirigami.Units.mediumSpacing
                    role: "caption"
                    font.family: "monospace"
                    wrapMode: Text.WrapAnywhere
                    verticalAlignment: Text.AlignVCenter
                    text: updateRow.update?.command ?? ""
                }
            }

            SmallButton {
                objectName: "updateCopy"
                Layout.alignment: Qt.AlignTop
                animated: updateRow.animated
                text: updateRow.updater.copied ? I18n.tr(updateRow.lang, "Copied") : I18n.tr(updateRow.lang, "Copy")
                onClicked: updateRow.updater.copyCommand()
            }
        }
    }
}
