pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import "logic/Options.js" as Options
import "logic/Preferences.js" as Preferences
import "logic/Settings.js" as Settings

SettingsGroup {
    id: group

    required property ConfigScaffold page
    required property bool capable
    required property real controlWidth
    readonly property var display: page.current.display
    readonly property string mode: capable ? display.panelMode : "headline"
    readonly property var limits: display.panelLimits
    property bool limitsOpen: true

    function setLimits(value, chosen) {
        page.setDisplay("panelLimits", Preferences.toggledLimits(limits, value, chosen));
    }

    title: page.tr("Panel")
    description: capable ? page.tr("Move the widget in panel edit mode.") : ""

    SettingsRow {
        visible: group.capable
        separated: false
        title: group.page.tr("Panel shows")
        subtitle: group.page.tr("What sits in the panel")

        OptionCombo {
            objectName: "panelMode"
            Layout.preferredWidth: group.controlWidth
            options: Options.panelModeOptions(group.page.lang)
            value: group.display.panelMode
            onPicked: value => group.page.setDisplay("panelMode", value)
        }
    }

    SettingsRow {
        visible: group.mode === "headline"
        separated: group.capable
        title: group.page.tr("Panel limit")
        subtitle: group.page.tr("The limit shown next to the clock")

        OptionCombo {
            Layout.preferredWidth: group.controlWidth
            options: Options.limitOptions(group.page.lang, group.page.snapshot, group.page.current.headline)
            value: Options.headlineKey(group.page.current.headline)
            onPicked: value => group.page.updateSettings(Settings.headlinePatch(Options.headlineFor(value)))
        }
    }

    SettingsRow {
        objectName: "panelLimitsRow"
        visible: group.mode === "several"
        title: group.page.tr("Limits in the panel")
        subtitle: Preferences.limitsSummary(group.page.lang, group.limits)

        IconButton {
            iconName: group.limitsOpen ? "arrow-up" : "arrow-down"
            text: group.page.tr("Limits in the panel")
            onClicked: group.limitsOpen = !group.limitsOpen
        }
    }

    Repeater {
        model: group.mode === "several" && group.limitsOpen ? Preferences.limitChoices(group.page.lang, group.page.snapshot, group.limits) : []

        ProviderSettingsRow {
            id: choiceRow

            required property var modelData
            readonly property bool chosen: Preferences.isChosen(group.limits, modelData.value)

            provider: Preferences.providerOf(modelData.value)
            title: modelData.label

            leading: QQC2.CheckBox {
                objectName: "panelLimitChoice"
                checked: choiceRow.chosen
                enabled: Preferences.canChoose(group.limits, choiceRow.modelData.value)
                onToggled: group.setLimits(choiceRow.modelData.value, checked)
            }
        }
    }

    SettingsRow {
        visible: group.capable && group.mode !== "icon"
        title: group.page.tr("Indicator")
        subtitle: group.page.tr("The mark in front of each figure")

        SegmentedControl {
            Layout.preferredWidth: group.controlWidth
            options: Options.panelIndicatorOptions(group.page.lang)
            current: group.display.panelIndicator
            onSelected: value => group.page.setDisplay("panelIndicator", value)
        }
    }

    SettingsRow {
        visible: group.mode !== "icon"
        title: group.page.tr("Panel label")

        SegmentedControl {
            Layout.preferredWidth: group.capable ? group.controlWidth * 1.25 : group.controlWidth
            options: Options.panelLabelOptions(group.page.lang, group.capable)
            current: group.display.panelLabel
            onSelected: value => group.page.setDisplay("panelLabel", value)
        }
    }
}
