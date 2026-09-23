import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Registry.js" as Registry
import "logic/Tokens.js" as Tokens

Item {
    id: addRow

    required property var provider
    required property string lang
    required property bool launched
    readonly property var method: provider.method
    readonly property string keyHint: Registry.keyHint(method)
    readonly property bool takesLabel: method.kind !== "auto_detect"
    readonly property real textIndent: Kirigami.Units.iconSizes.smallMedium + Kirigami.Units.largeSpacing

    signal addRequested(string label)

    function tr(msgid, values) {
        return I18n.tr(lang, msgid, values);
    }

    function actionText() {
        if (method.kind === "api_key")
            return tr("Add API key…");
        if (method.kind === "auto_detect")
            return tr("Rescan");
        return tr("Sign in…");
    }

    function launchedText() {
        if (method.kind === "api_key")
            return tr("A terminal window opened. Paste the API key there; the account shows up here when it is done.");
        if (method.kind === "auto_detect")
            return tr("Looking for accounts. New ones show up here in a moment.");
        return tr("A terminal window opened. Finish signing in there; the account shows up here when it is done.");
    }

    Layout.fillWidth: true
    implicitHeight: column.implicitHeight + Kirigami.Units.largeSpacing * 2

    Rectangle {
        x: Metrics.rowInset(Kirigami.Units)
        width: parent.width - x * 2
        height: Metrics.hairline(Kirigami.Units)
        color: Tokens.separator(Kirigami.Theme)
    }

    ColumnLayout {
        id: column

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: Metrics.rowInset(Kirigami.Units)
        anchors.rightMargin: Metrics.rowInset(Kirigami.Units)
        spacing: Kirigami.Units.smallSpacing

        RowLayout {
            spacing: Kirigami.Units.largeSpacing

            ProviderIcon {
                Layout.alignment: Qt.AlignTop
                provider: addRow.provider.id
                implicitWidth: Kirigami.Units.iconSizes.smallMedium
                implicitHeight: implicitWidth
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Math.round(Kirigami.Units.smallSpacing / 2)

                TextLabel {
                    Layout.fillWidth: true
                    role: "label"
                    weight: Font.Medium
                    text: addRow.tr("Add {provider} account", {
                        provider: addRow.provider.name
                    })
                    elide: Text.ElideRight
                }

                TextLabel {
                    Layout.fillWidth: true
                    role: "caption"
                    emphasis: "secondary"
                    text: Registry.methodHint(addRow.lang, addRow.provider)
                    wrapMode: Text.Wrap
                }
            }

            QQC2.TextField {
                id: labelField

                visible: addRow.takesLabel
                Layout.preferredWidth: Kirigami.Units.gridUnit * 8
                placeholderText: addRow.tr("Label (optional)")
                maximumLength: 64
                onAccepted: addRow.addRequested(text)
            }

            SmallButton {
                text: addRow.actionText()
                onClicked: addRow.addRequested(addRow.takesLabel ? labelField.text : "")
            }
        }

        RowLayout {
            visible: addRow.method.kind === "api_key" && (addRow.keyHint !== "" || addRow.method.consoleUrl !== null)
            Layout.leftMargin: addRow.textIndent
            spacing: Kirigami.Units.largeSpacing

            TextLabel {
                Layout.fillWidth: true
                role: "caption"
                emphasis: "secondary"
                wrapMode: Text.Wrap
                text: addRow.keyHint
            }

            SmallButton {
                visible: addRow.method.consoleUrl !== null
                text: addRow.tr("Get a key…")
                onClicked: Qt.openUrlExternally(addRow.method.consoleUrl)
            }
        }

        TextLabel {
            visible: addRow.launched
            Layout.fillWidth: true
            Layout.leftMargin: addRow.textIndent
            role: "caption"
            emphasis: "secondary"
            wrapMode: Text.Wrap
            text: addRow.launchedText()
        }
    }
}
