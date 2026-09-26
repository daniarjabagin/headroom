import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import "logic/Options.js" as Options
import "logic/Preferences.js" as Preferences

SettingsGroup {
    id: group

    required property ConfigScaffold page
    required property real controlWidth
    readonly property var display: page.current.display

    title: page.tr("Spend")

    SettingsRow {
        separated: false
        title: group.page.tr("Show spend")
        subtitle: group.page.tr("Spend ring for all tools at the top of the popup")

        QQC2.Switch {
            objectName: "showSpend"
            checked: group.display.showSpend
            onToggled: group.page.setDisplay("showSpend", checked)
        }
    }

    SettingsRow {
        title: group.page.tr("Show models and projects")
        subtitle: group.page.tr("Top models and projects under the spend ring")

        QQC2.Switch {
            objectName: "showBreakdown"
            checked: group.display.showBreakdown
            onToggled: group.page.setDisplay("showBreakdown", checked)
        }
    }

    SettingsRow {
        title: group.page.tr("Default period")
        subtitle: group.page.tr("The tab the spend ring opens on")

        OptionCombo {
            objectName: "spendPeriod"
            Layout.preferredWidth: group.controlWidth
            options: Preferences.spendPeriodOptions(group.page.lang)
            value: group.display.spendPeriod
            onPicked: value => group.page.setDisplay("spendPeriod", value)
        }
    }

    SettingsRow {
        title: group.page.tr("Units")

        SegmentedControl {
            Layout.preferredWidth: group.controlWidth * 1.25
            options: Options.spendUnitOptions(group.page.lang)
            current: group.display.spendUnit
            onSelected: value => group.page.setDisplay("spendUnit", value)
        }
    }

    SettingsRow {
        title: group.page.tr("Breakdown on hover")
        subtitle: group.page.tr("What a spend legend row splits into")

        SegmentedControl {
            Layout.preferredWidth: group.controlWidth
            options: Options.spendBreakdownOptions(group.page.lang)
            current: group.display.spendBreakdown
            onSelected: value => group.page.setDisplay("spendBreakdown", value)
        }
    }
}
