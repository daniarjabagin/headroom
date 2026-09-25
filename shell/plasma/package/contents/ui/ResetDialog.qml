import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import "logic/I18n.js" as I18n

Kirigami.PromptDialog {
    id: dialog

    required property string lang

    signal resetConfirmed

    title: I18n.tr(lang, "Reset all settings?")
    subtitle: I18n.tr(lang, "Accounts stay signed in; appearance, notifications and hidden limits return to defaults.")
    standardButtons: QQC2.Dialog.Cancel
    customFooterActions: [
        Kirigami.Action {
            text: I18n.tr(dialog.lang, "Reset")
            icon.name: "edit-reset"
            onTriggered: {
                dialog.resetConfirmed();
                dialog.close();
            }
        }
    ]
}
