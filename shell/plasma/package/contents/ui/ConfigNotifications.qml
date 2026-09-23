pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import "logic/Options.js" as Options
import "logic/Settings.js" as Settings

ConfigScaffold {
    id: page

    SettingsGroup {
        title: page.tr("Notify Me When")
        description: page.tr("Hidden accounts and hidden limits never notify.")

        Repeater {
            model: Options.MILESTONES

            SettingsRow {
                id: milestoneRow

                required property var modelData
                required property int index

                separated: index > 0
                title: page.tr(modelData.title)
                subtitle: page.tr(modelData.subtitle)

                QQC2.Switch {
                    checked: page.current.notifications[milestoneRow.modelData.key]
                    onToggled: page.updateSettings(raw => Settings.patchNotifications(raw, {
                            [milestoneRow.modelData.key]: checked
                        }))
                }
            }
        }
    }
}
