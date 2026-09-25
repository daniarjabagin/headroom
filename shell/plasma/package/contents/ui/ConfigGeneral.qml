pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Options.js" as Options
import "logic/Settings.js" as Settings
import "logic/Update.js" as Update
import "logic/UpdateCheck.js" as UpdateCheck

ConfigScaffold {
    id: page

    readonly property var display: current.display
    readonly property real controlWidth: Kirigami.Units.gridUnit * 13
    readonly property int clockTickMs: 30000
    property var now: new Date()
    property bool checking: false
    property var checkResult: null
    readonly property var shownCheck: snapshot !== null ? UpdateCheck.shown(snapshot, checkResult) : null

    function checkNow() {
        if (checking)
            return;
        checking = true;
        const previous = snapshot?.updateCheck?.checkedAt ?? null;
        daemon.checkForUpdates((errorName, value) => {
            page.checkResult = UpdateCheck.outcome(errorName, value, previous);
            page.now = new Date();
            page.checking = false;
        });
    }

    function releaseActionLabel() {
        if (updater.kind === "command")
            return updater.copied ? tr("Copied") : tr("Copy");
        return Update.actionLabel(lang, updater.kind);
    }

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
                onPicked: value => page.updateSettings(Settings.headlinePatch(Options.headlineFor(value)))
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
    }

    SettingsGroup {
        title: page.tr("Updates")

        SettingsRow {
            separated: false
            title: page.tr("Check for updates")
            subtitle: page.tr("Once a day, asks GitHub for the latest release. Nothing else is sent.")

            QQC2.Switch {
                objectName: "updatesCheck"
                checked: page.current.updates.check
                onToggled: page.updateSettings(Settings.updatesPatch(checked))
            }
        }

        SettingsRow {
            objectName: "updateCheckRow"
            visible: UpdateCheck.visible(page.current.updates.check, page.snapshot)
            title: page.shownCheck !== null ? UpdateCheck.statusLine(page.lang, page.shownCheck, page.snapshot.appVersion, page.now) : ""

            QQC2.BusyIndicator {
                objectName: "updateCheckBusy"
                visible: page.checking
                running: visible
            }

            QQC2.Button {
                objectName: "updateCheckNow"
                enabled: !page.checking
                text: page.tr("Check now")
                onClicked: page.checkNow()
            }
        }

        SettingsRow {
            id: releaseRow

            readonly property var run: updater.kind === "install" ? updater.run : Update.IDLE

            objectName: "updateRelease"
            visible: updater.update !== null
            title: updater.update !== null ? Update.title(page.lang, updater.update) : ""
            subtitle: Update.runLine(page.lang, run) || (updater.kind === "command" ? updater.update.command : "")

            QQC2.Button {
                visible: updater.update !== null && Update.showsWhatsNew(updater.update) && releaseRow.run.phase === "idle"
                flat: true
                text: page.tr("What's new")
                onClicked: updater.openRelease()
            }

            QQC2.BusyIndicator {
                visible: releaseRow.run.phase === "running"
                running: visible
            }

            QQC2.Button {
                objectName: "updateReleaseAction"
                visible: updater.kind !== "" && (releaseRow.run.phase === "idle" || releaseRow.run.phase === "failed")
                highlighted: updater.kind === "install" && releaseRow.run.phase === "idle"
                text: releaseRow.run.phase === "failed" ? page.tr("Retry") : page.releaseActionLabel()
                onClicked: updater.trigger()
            }
        }
    }

    Timer {
        interval: page.clockTickMs
        repeat: true
        running: page.visible
        onTriggered: page.now = new Date()
    }

    UpdateActions {
        id: updater

        update: page.snapshot?.update ?? null
    }
}
