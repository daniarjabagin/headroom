import QtQuick
import QtQuick.Controls as QQC2
import "logic/NotificationPrefs.js" as NotificationPrefs

SettingsGroup {
    id: group

    required property ConfigScaffold page
    readonly property var hours: page.current.notifications.quietHours

    function saveClock(key, text) {
        const patch = NotificationPrefs.clockPatch(hours, key, text);
        if (patch !== null)
            page.updateSettings(patch);
    }

    title: page.tr("Quiet hours")
    description: page.tr("Held notifications arrive together when quiet hours end.")

    SettingsRow {
        separated: false
        title: group.page.tr("Quiet hours")
        subtitle: group.page.tr("Hold notifications while you are away")

        QQC2.Switch {
            objectName: "quietEnabled"
            checked: group.hours.enabled
            enabled: group.hours.enabled || NotificationPrefs.canEnable(group.hours)
            onToggled: group.page.updateSettings(NotificationPrefs.quietPatch({
                enabled: checked
            }))
        }
    }

    SettingsRow {
        title: group.page.tr("From")

        ClockField {
            objectName: "quietFrom"
            clock: group.hours.from
            onCommitted: text => group.saveClock("from", text)
        }
    }

    SettingsRow {
        title: group.page.tr("To")

        ClockField {
            objectName: "quietTo"
            clock: group.hours.to
            onCommitted: text => group.saveClock("to", text)
        }
    }

    SettingsRow {
        title: group.page.tr("Still show critical alerts")
        subtitle: group.page.tr("Will run out and Almost out come through")

        QQC2.Switch {
            objectName: "quietCritical"
            checked: group.hours.allowCritical
            onToggled: group.page.updateSettings(NotificationPrefs.quietPatch({
                allowCritical: checked
            }))
        }
    }
}
