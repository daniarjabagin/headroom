pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Options.js" as Options
import "logic/Settings.js" as Settings

ConfigScaffold {
    id: page

    readonly property var display: current.display
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
            title: page.tr("Translucent popup")
            subtitle: page.tr("Let the blurred desktop show through the popup")

            QQC2.Switch {
                checked: page.display.translucent
                onToggled: page.setDisplay("translucent", checked)
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
    }

    SettingsGroup {
        title: page.tr("Top Panel")

        SettingsRow {
            separated: false
            title: page.tr("Panel limit")
            subtitle: page.tr("The limit shown next to the clock")

            OptionCombo {
                Layout.preferredWidth: page.controlWidth
                options: Options.limitOptions(page.lang, page.snapshot, page.current.headline)
                value: Options.headlineKey(page.current.headline)
                onPicked: value => page.updateSettings(raw => Settings.withHeadline(raw, Options.headlineFor(value)))
            }
        }

        SettingsRow {
            title: page.tr("Panel label")

            SegmentedControl {
                Layout.preferredWidth: page.controlWidth
                options: Options.panelLabelOptions(page.lang)
                current: page.display.panelLabel
                onSelected: value => page.setDisplay("panelLabel", value)
            }
        }
    }

    SettingsGroup {
        title: page.tr("Sections")

        Repeater {
            model: Options.SECTIONS

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

    SettingsGroup {
        title: page.tr("Updates")

        SettingsRow {
            separated: false
            title: page.tr("Refresh interval")
            subtitle: page.tr("How often the service asks each provider")

            OptionCombo {
                Layout.preferredWidth: page.controlWidth
                options: Options.refreshOptions(page.lang, page.current.refreshIntervalSecs)
                value: page.current.refreshIntervalSecs
                onPicked: value => page.updateSettings(raw => Settings.withRefreshInterval(raw, value))
            }
        }
    }
}
