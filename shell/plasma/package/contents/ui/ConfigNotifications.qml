pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/NotificationPrefs.js" as NotificationPrefs
import "logic/Options.js" as Options
import "logic/Settings.js" as Settings

ConfigScaffold {
    id: page

    readonly property var notifications: current.notifications
    readonly property bool capable: daemon.supports06
    readonly property real controlWidth: Kirigami.Units.gridUnit * 13
    readonly property var providerRows: NotificationPrefs.providerRows(snapshot)
    property bool providersOpen: false

    SettingsGroup {
        title: page.tr("Notify Me When")
        description: page.tr("Hidden accounts and hidden limits never notify.")

        Repeater {
            model: Options.MILESTONES

            SettingsRow {
                id: milestoneRow

                required property var modelData
                required property int index

                objectName: `milestone-${modelData.key}`
                separated: index > 0
                title: page.tr(modelData.title)
                subtitle: NotificationPrefs.milestoneSubtitle(page.lang, modelData, page.notifications.thresholdPercent, page.capable)

                QQC2.Switch {
                    checked: page.notifications[milestoneRow.modelData.key]
                    onToggled: page.updateSettings(Settings.notificationsPatch({
                        [milestoneRow.modelData.key]: checked
                    }))
                }
            }
        }
    }

    SettingsGroup {
        visible: page.capable
        title: page.tr("Alert threshold")

        SettingsRow {
            separated: false
            title: page.tr("Alert when less than")
            subtitle: page.tr("Used by Almost out")

            SegmentedControl {
                objectName: "thresholdControl"
                Layout.preferredWidth: page.controlWidth
                options: Options.thresholdOptions(page.lang, page.notifications.thresholdPercent)
                current: page.notifications.thresholdPercent
                onSelected: value => page.updateSettings(NotificationPrefs.thresholdPatch(value))
            }
        }

        SettingsRow {
            objectName: "perProviderRow"
            visible: page.providerRows.length > 0
            title: page.tr("Per provider")
            subtitle: NotificationPrefs.perProviderSummary(page.lang, page.notifications.providerThresholds, page.providerRows)

            IconButton {
                objectName: "perProviderToggle"
                iconName: page.providersOpen ? "arrow-up" : "arrow-down"
                text: page.tr("Per provider")
                onClicked: page.providersOpen = !page.providersOpen
            }
        }

        Repeater {
            model: page.providersOpen ? page.providerRows.length : 0

            ProviderSettingsRow {
                id: providerRow

                required property int index
                readonly property var modelData: page.providerRows[index]
                readonly property var saved: page.notifications.providerThresholds[modelData.provider]

                provider: modelData.provider
                title: modelData.name

                OptionCombo {
                    objectName: `threshold-${providerRow.modelData.provider}`
                    Layout.preferredWidth: page.controlWidth * 0.75
                    options: Options.providerThresholdOptions(page.lang, page.notifications.thresholdPercent, providerRow.saved)
                    value: Options.providerThresholdKey(page.notifications.providerThresholds, providerRow.modelData.provider)
                    onPicked: value => page.updateSettings(NotificationPrefs.providerPatch(providerRow.modelData.provider, value))
                }
            }
        }
    }

    QuietHoursGroup {
        visible: page.capable
        page: page
    }
}
