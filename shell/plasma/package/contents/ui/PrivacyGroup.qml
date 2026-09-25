import QtQuick
import QtQuick.Controls as QQC2
import "logic/Settings.js" as Settings
import "logic/Update.js" as Update
import "logic/UpdateCheck.js" as UpdateCheck

SettingsGroup {
    id: group

    required property ConfigScaffold page
    required property bool capable
    readonly property var snapshot: page.snapshot
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
        page.daemon.checkForUpdates((errorName, value) => {
            group.checkResult = UpdateCheck.outcome(errorName, value, previous);
            group.now = new Date();
            group.checking = false;
        });
    }

    function releaseActionLabel() {
        if (updater.kind === "command")
            return updater.copied ? page.tr("Copied") : page.tr("Copy");
        return Update.actionLabel(page.lang, updater.kind);
    }

    title: capable ? page.tr("Privacy") : page.tr("Updates")

    SettingsRow {
        separated: false
        title: group.page.tr("Check for updates")
        subtitle: group.page.tr("Once a day, asks GitHub for the latest release. Nothing else is sent.")

        QQC2.Switch {
            objectName: "updatesCheck"
            checked: group.page.current.updates.check
            onToggled: group.page.updateSettings(Settings.updatesPatch(checked))
        }
    }

    SettingsRow {
        objectName: "updateCheckRow"
        visible: UpdateCheck.visible(group.page.current.updates.check, group.snapshot)
        title: group.shownCheck !== null ? UpdateCheck.statusLine(group.page.lang, group.shownCheck, group.snapshot.appVersion, group.now) : ""

        QQC2.BusyIndicator {
            objectName: "updateCheckBusy"
            visible: group.checking
            running: visible
        }

        QQC2.Button {
            objectName: "updateCheckNow"
            enabled: !group.checking
            text: group.page.tr("Check now")
            onClicked: group.checkNow()
        }
    }

    SettingsRow {
        id: releaseRow

        readonly property var run: updater.kind === "install" ? updater.run : Update.IDLE

        objectName: "updateRelease"
        visible: updater.update !== null
        title: updater.update !== null ? Update.title(group.page.lang, updater.update) : ""
        subtitle: Update.runLine(group.page.lang, run) || (updater.kind === "command" ? updater.update.command : "")

        QQC2.Button {
            visible: updater.update !== null && Update.showsWhatsNew(updater.update) && releaseRow.run.phase === "idle"
            flat: true
            text: group.page.tr("What's new")
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
            text: releaseRow.run.phase === "failed" ? group.page.tr("Retry") : group.releaseActionLabel()
            onClicked: updater.trigger()
        }
    }

    SettingsRow {
        visible: group.capable
        title: group.page.tr("Status pages")
        subtitle: group.page.tr("Show provider incidents from public status pages")

        QQC2.Switch {
            objectName: "statusPages"
            checked: group.page.current.statusPages.enabled
            onToggled: group.page.updateSettings(Settings.statusPagesPatch(checked))
        }
    }

    Timer {
        interval: group.clockTickMs
        repeat: true
        running: group.visible
        onTriggered: group.now = new Date()
    }

    UpdateActions {
        id: updater

        visible: false
        update: group.snapshot?.update ?? null
    }
}
