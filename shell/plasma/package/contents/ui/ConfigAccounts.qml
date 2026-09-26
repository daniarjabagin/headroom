pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/AccountList.js" as AccountList
import "logic/Commands.js" as Commands
import "logic/I18n.js" as I18n
import "logic/Motion.js" as Motion
import "logic/Order.js" as Order
import "logic/Registry.js" as Registry
import "logic/Settings.js" as Settings

ConfigScaffold {
    id: page

    trackProviders: true
    readonly property var accounts: snapshot?.accounts ?? []
    readonly property var providers: daemon.providers ?? []
    readonly property bool animated: Motion.enabled(Kirigami.Units, current.reducedMotion)
    property string selectedId: ""
    property bool adding: false
    readonly property var selectedAccount: AccountList.selected(accounts, selectedId)
    readonly property bool showsAdd: adding || selectedAccount === null
    property string launchedProvider: ""
    property var pendingKinds: ({})

    function select(accountId) {
        selectedId = accountId;
        adding = false;
    }

    function move(accountId, offset) {
        const ids = accounts.map(account => account.id);
        const from = ids.indexOf(accountId);
        selectedId = accountId;
        daemon.setOrder(Order.moveItem(ids, from, from + offset));
    }

    function setWindowHidden(accountId, windowId, hidden) {
        updateSettings(Settings.displayPatch(Settings.windowHiddenPatch(current.display, accountId, windowId, hidden)));
    }

    function setStarred(accountId, starred) {
        updateSettings(Settings.displayPatch(Settings.starredPatch(current.display, accountId, starred)));
    }

    function launch(command, kind) {
        pendingKinds = Commands.track(pendingKinds, command, kind);
        runner.run(command);
    }

    function showCommandError(error) {
        if (!Commands.isCommandError(error))
            throw error;
        message = I18n.errorText(lang, error);
    }

    function add(provider, label) {
        message = "";
        try {
            const plan = Commands.addPlan(provider, label, tr("Press Enter to close this window"));
            launchedProvider = provider.id;
            if (plan.kind === "terminal")
                launch(plan.command, "add");
            else
                daemon.restoreAccounts(provider.id);
        } catch (error) {
            showCommandError(error);
        }
    }

    function signIn(accountId) {
        message = "";
        try {
            launch(Commands.loginAccountCommand(accountId, tr("Press Enter to close this window")), "add");
        } catch (error) {
            showCommandError(error);
        }
    }

    function remove(accountId) {
        message = "";
        try {
            launch(Commands.removeAccountCommand(accountId), "remove");
        } catch (error) {
            showCommandError(error);
        }
    }

    function removeFinished(exitCode, stdout) {
        const outcome = Commands.progressOutcome(stdout, exitCode);
        message = outcome.ok ? "" : (outcome.message || tr("Couldn't remove the account"));
    }

    function addFinished(exitCode) {
        if (exitCode === 127)
            message = tr("No terminal found. Install xdg-terminal-exec or Konsole, or run \"headroom accounts add\" yourself.");
        launchedProvider = "";
    }

    function commandExited(command, exitCode, stdout) {
        const settled = Commands.settle(pendingKinds, command);
        pendingKinds = settled.pending;
        if (settled.kind === "remove")
            removeFinished(exitCode, stdout);
        else if (settled.kind === "add")
            addFinished(exitCode);
        daemon.rescan();
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: Kirigami.Units.largeSpacing

        AccountListPane {
            Layout.preferredWidth: Kirigami.Units.gridUnit * 13
            Layout.maximumWidth: Layout.preferredWidth
            Layout.alignment: Qt.AlignTop
            accounts: page.accounts
            display: page.current.display
            lang: page.lang
            selectedId: page.selectedAccount?.id ?? ""
            adding: page.showsAdd
            animated: page.animated
            onAccountSelected: accountId => page.select(accountId)
            onAddRequested: page.adding = true
        }

        Loader {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            active: !page.showsAdd
            visible: active

            sourceComponent: AccountDetailPane {
                account: page.selectedAccount
                display: page.current.display
                links: Registry.providerLinks(page.providers, page.selectedAccount.provider)
                lang: page.lang
                position: AccountList.position(page.accounts, page.selectedAccount.id)
                count: page.accounts.length
                canStar: page.daemon.supports06
                animated: page.animated
                onHiddenToggled: hidden => page.daemon.setHidden(page.selectedAccount.id, hidden)
                onStarToggled: starred => page.setStarred(page.selectedAccount.id, starred)
                onLabelApplied: label => page.daemon.setLabel(page.selectedAccount.id, label)
                onMoveRequested: offset => page.move(page.selectedAccount.id, offset)
                onWindowToggled: (windowId, hidden) => page.setWindowHidden(page.selectedAccount.id, windowId, hidden)
                onSignInRequested: accountId => page.signIn(accountId)
                onLinkOpened: url => Qt.openUrlExternally(url)
                onRemoveConfirmed: page.remove(page.selectedAccount.id)
            }
        }

        AddAccountPane {
            visible: page.showsAdd
            Layout.alignment: Qt.AlignTop
            providers: page.providers
            lang: page.lang
            providersRequested: page.daemon.providersRequested
            launchedProvider: page.launchedProvider
            animated: page.animated
            onProviderPicked: page.launchedProvider = ""
            onAddRequested: (provider, label) => page.add(provider, label)
        }
    }

    CommandRunner {
        id: runner

        onExited: (command, exitCode, stdout) => page.commandExited(command, exitCode, stdout)
    }
}
