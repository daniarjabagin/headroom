pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Options.js" as Options
import "logic/Preferences.js" as Preferences
import "logic/Settings.js" as Settings

ConfigScaffold {
    id: page

    readonly property var display: current.display
    readonly property bool capable: daemon.supports06
    readonly property real controlWidth: Kirigami.Units.gridUnit * 13

    SettingsGroup {
        title: page.tr("Appearance")

        SettingsRow {
            separated: false
            title: page.tr("Theme")
            subtitle: page.tr("The popup follows the system unless you pick one")

            SegmentedControl {
                Layout.preferredWidth: page.controlWidth
                options: Options.themeOptions(page.lang)
                current: page.display.theme
                onSelected: value => page.setDisplay("theme", value)
            }
        }

        SettingsRow {
            title: page.tr("Language")

            SegmentedControl {
                Layout.preferredWidth: page.controlWidth
                options: Options.languageOptions(page.lang)
                current: page.display.language
                onSelected: value => page.setDisplay("language", value)
            }
        }

        SettingsRow {
            objectName: "densityRow"
            visible: page.capable
            title: page.tr("Density")
            subtitle: page.tr("Compact fits more accounts into the popup")

            SegmentedControl {
                Layout.preferredWidth: page.controlWidth
                options: Options.densityOptions(page.lang)
                current: page.display.density
                onSelected: value => page.setDisplay("density", value)
            }
        }

        SettingsRow {
            visible: page.capable
            title: page.tr("Time format")

            SegmentedControl {
                Layout.preferredWidth: page.controlWidth
                options: Options.timeFormatOptions(page.lang)
                current: page.display.timeFormat
                onSelected: value => page.setDisplay("timeFormat", value)
            }
        }

        SettingsRow {
            title: page.tr("Translucent popup")
            subtitle: page.tr("Let the blurred desktop show through the popup")

            QQC2.Switch {
                checked: page.display.translucent
                onToggled: page.setDisplay("translucent", checked)
            }
        }

        SettingsRow {
            title: page.tr("Reduce motion")
            subtitle: page.tr("Skip popup and meter animations")

            QQC2.Switch {
                objectName: "reducedMotion"
                checked: page.current.reducedMotion
                onToggled: page.updateSettings(Preferences.reducedMotionPatch(checked))
            }
        }
    }

    SettingsGroup {
        title: page.tr("Popup")

        SettingsRow {
            separated: false
            title: page.tr("Show values as")
            subtitle: page.tr("Click a reading in the popup to switch")

            SegmentedControl {
                Layout.preferredWidth: page.controlWidth
                options: Options.valueModeOptions(page.lang)
                current: page.display.valueMode
                onSelected: value => page.setDisplay("valueMode", value)
            }
        }

        SettingsRow {
            title: page.tr("Reset time")
            subtitle: page.tr("Click a reset time in the popup to switch")

            SegmentedControl {
                Layout.preferredWidth: page.controlWidth
                options: Options.resetFormatOptions(page.lang)
                current: page.display.resetFormat
                onSelected: value => page.setDisplay("resetFormat", value)
            }
        }

        SettingsRow {
            title: page.tr("Combine accounts of the same provider")
            subtitle: page.tr("Show one card per provider and add up the limits of its accounts")

            QQC2.Switch {
                objectName: "combineAccounts"
                checked: page.display.combineAccounts
                onToggled: page.setDisplay("combineAccounts", checked)
            }
        }
    }

    PanelGroup {
        page: page
        capable: page.capable
        controlWidth: page.controlWidth
    }

    SpendGroup {
        visible: page.capable
        page: page
        controlWidth: page.controlWidth
    }

    SettingsGroup {
        title: page.tr("Sections")

        Repeater {
            model: Preferences.sections(page.capable)

            SettingsRow {
                id: sectionRow

                required property var modelData
                required property int index

                separated: index > 0
                title: page.tr(modelData.title)
                subtitle: page.tr(modelData.subtitle)

                QQC2.Switch {
                    checked: page.display[sectionRow.modelData.key]
                    onToggled: page.setDisplay(sectionRow.modelData.key, checked)
                }
            }
        }
    }

    CardsGroup {
        visible: page.capable
        page: page
    }

    SettingsGroup {
        title: page.tr("Data refresh")

        SettingsRow {
            separated: false
            title: page.tr("Refresh interval")
            subtitle: page.tr("How often the service asks each provider")

            OptionCombo {
                Layout.preferredWidth: page.controlWidth
                options: Options.refreshOptions(page.lang, page.current.refreshIntervalSecs)
                value: page.current.refreshIntervalSecs
                onPicked: value => page.updateSettings(Settings.refreshIntervalPatch(value))
            }
        }

        SettingsRow {
            visible: page.capable
            title: page.tr("Faster while coding tools run")
            subtitle: page.tr("Every minute while Claude Code, Codex or Cursor is open")

            QQC2.Switch {
                objectName: "adaptiveRefresh"
                checked: page.current.adaptiveRefresh
                onToggled: page.updateSettings(Settings.adaptiveRefreshPatch(checked))
            }
        }
    }

    PrivacyGroup {
        page: page
        capable: page.capable
    }

    ShortcutGroup {
        visible: page.capable
        page: page
    }
}
