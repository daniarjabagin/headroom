pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import "logic/Format.js" as Format
import "logic/I18n.js" as I18n
import "logic/Settings.js" as Settings

SettingsGroup {
    id: group

    required property var account
    required property var display
    required property string lang

    signal windowToggled(string windowId, bool hidden)

    visible: account.windows.length > 0
    title: I18n.tr(lang, "Limits")
    description: I18n.tr(lang, "Hidden limits leave the popup, the panel and notifications.")

    Repeater {
        model: group.account.windows

        SettingsRow {
            id: windowRow

            required property int index
            required property var modelData
            readonly property var window: modelData

            separated: index > 0
            title: Format.windowLabel(group.lang, window)
            subtitle: window.remainingPercent === null ? I18n.tr(group.lang, "No data yet") : Format.percentLeft(group.lang, window.remainingPercent)

            QQC2.Switch {
                objectName: "windowSwitch"
                checked: !Settings.isWindowHidden(group.display, group.account.id, windowRow.window.id)
                onToggled: group.windowToggled(windowRow.window.id, !checked)
            }
        }
    }
}
