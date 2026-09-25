pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import "logic/Preferences.js" as Preferences
import "logic/Settings.js" as Settings

SettingsGroup {
    id: group

    required property ConfigScaffold page
    readonly property var display: page.current.display
    readonly property var starRows: Preferences.starRows(page.snapshot)

    title: page.tr("Popup cards")
    description: page.tr("Star the accounts that always show their limits.")

    SettingsRow {
        separated: false
        title: group.page.tr("Collapse unstarred accounts")
        subtitle: group.page.tr("They fold into one line in the popup and open on click")

        QQC2.Switch {
            objectName: "collapseUnstarred"
            checked: group.display.collapseUnstarred
            onToggled: group.page.setDisplay("collapseUnstarred", checked)
        }
    }

    Repeater {
        model: group.starRows.length

        ProviderSettingsRow {
            id: starRow

            required property int index
            readonly property var modelData: group.starRows[index]
            readonly property bool starred: Settings.isStarred(group.display, modelData.id)

            provider: modelData.provider
            title: modelData.title
            subtitle: modelData.subtitle

            TextLabel {
                role: "caption"
                emphasis: "secondary"
                text: Preferences.starLabel(group.page.lang, starRow.starred)
            }

            IconButton {
                objectName: "starButton"
                iconName: starRow.starred ? "starred-symbolic" : "non-starred-symbolic"
                text: Preferences.starLabel(group.page.lang, starRow.starred)
                onClicked: group.page.updateSettings(Settings.displayPatch(Settings.starredPatch(group.display, starRow.modelData.id, !starRow.starred)))
            }
        }
    }
}
