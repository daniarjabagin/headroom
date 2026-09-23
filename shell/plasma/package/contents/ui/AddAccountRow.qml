import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Providers.js" as Providers
import "logic/Tokens.js" as Tokens

Item {
    id: addRow

    required property string provider
    required property string lang
    required property bool separated
    required property bool launched

    signal signInRequested(string provider, string label)

    Layout.fillWidth: true
    implicitHeight: column.implicitHeight + Kirigami.Units.largeSpacing * 2

    Rectangle {
        visible: addRow.separated
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
                provider: addRow.provider
                implicitWidth: Kirigami.Units.iconSizes.smallMedium
                implicitHeight: implicitWidth
            }

            TextLabel {
                Layout.fillWidth: true
                role: "label"
                weight: Font.Medium
                text: I18n.tr(addRow.lang, "Add {provider} account", {
                    provider: Providers.providerInfo(addRow.provider).name
                })
                elide: Text.ElideRight
            }

            QQC2.TextField {
                id: labelField

                Layout.preferredWidth: Kirigami.Units.gridUnit * 8
                placeholderText: I18n.tr(addRow.lang, "Label (optional)")
                maximumLength: 64
                onAccepted: addRow.signInRequested(addRow.provider, text)
            }

            SmallButton {
                text: I18n.tr(addRow.lang, "Sign in…")
                onClicked: addRow.signInRequested(addRow.provider, labelField.text)
            }
        }

        TextLabel {
            visible: addRow.launched
            Layout.fillWidth: true
            Layout.leftMargin: Kirigami.Units.iconSizes.smallMedium + Kirigami.Units.largeSpacing
            role: "caption"
            emphasis: "secondary"
            wrapMode: Text.Wrap
            text: I18n.tr(addRow.lang, "A terminal window opened. Finish signing in there; the account shows up here when it is done.")
        }
    }
}
