pragma ComponentBehavior: Bound

import QtQuick
import "logic/Commands.js" as Commands
import "logic/Order.js" as Order
import "logic/Settings.js" as Settings

ConfigScaffold {
    id: page

    readonly property var accounts: snapshot?.accounts ?? []
    property var expandedIds: []
    property string launchedProvider: ""
    property int dragIndex: -1
    property real dragOffset: 0
    property int dropTarget: -1
    readonly property var dropSlot: Order.indicatorSlot(dragIndex, dropTarget, accounts.length)

    function toggleExpanded(accountId) {
        expandedIds = expandedIds.includes(accountId) ? expandedIds.filter(id => id !== accountId) : expandedIds.concat([accountId]);
    }

    function centers() {
        const found = [];
        for (let index = 0; index < rows.count; index++) {
            const item = rows.itemAt(index);
            found.push(item ? item.y + item.height / 2 : 0);
        }
        return found;
    }

    function dragMoved(index, offset) {
        const item = rows.itemAt(index);
        if (!item)
            return;
        dragIndex = index;
        dragOffset = offset;
        dropTarget = Order.targetIndex(centers(), index, item.y + item.height / 2 + offset);
    }

    function dragFinished(index) {
        const target = dropTarget;
        dragIndex = -1;
        dragOffset = 0;
        dropTarget = -1;
        if (target < 0 || target === index)
            return;
        const ids = accounts.map(account => account.id);
        daemon.setOrder(Order.moveItem(ids, index, target));
    }

    function setWindowHidden(accountId, windowId, hidden) {
        updateSettings(Settings.displayPatch(Settings.windowHiddenPatch(current.display, accountId, windowId, hidden)));
    }

    function signIn(provider, label) {
        message = "";
        launchedProvider = provider;
        runner.run(Commands.addAccountCommand(provider, label, tr("Press Enter to close this window")));
    }

    function remove(accountId) {
        message = "";
        try {
            runner.run(Commands.removeAccountCommand(accountId));
        } catch (error) {
            if (!Commands.isCommandError(error))
                throw error;
            message = error.message;
        }
    }

    function commandExited(command, exitCode, stdout) {
        if (command.includes("accounts remove")) {
            const outcome = Commands.progressOutcome(stdout, exitCode);
            message = outcome.ok ? "" : (outcome.message || tr("Couldn't remove the account"));
        } else if (exitCode === 127) {
            message = tr("No terminal found. Install xdg-terminal-exec or Konsole, or run \"headroom accounts add\" yourself.");
        }
        launchedProvider = "";
        daemon.rescan();
    }

    SettingsGroup {
        title: page.tr("Accounts")
        description: page.tr("Drag to reorder. Hidden accounts keep updating but leave the panel and notifications.")

        SettingsRow {
            visible: page.accounts.length === 0
            separated: false
            title: page.tr("No accounts yet")
            subtitle: page.tr("Sign in with the Codex or Claude Code CLI, or add an account below.")
        }

        Repeater {
            id: rows

            model: page.accounts.length

            AccountConfigRow {
                required property int index

                account: page.accounts[index]
                display: page.current.display
                lang: page.lang
                separated: index > 0
                expanded: page.expandedIds.includes(account.id)
                canReorder: page.accounts.length > 1
                lifted: page.dragIndex === index
                dragOffset: lifted ? page.dragOffset : 0
                indicator: page.dropSlot?.index === index ? (page.dropSlot.below ? "below" : "above") : ""
                onHiddenToggled: hidden => page.daemon.setHidden(account.id, hidden)
                onLabelApplied: label => page.daemon.setLabel(account.id, label)
                onWindowToggled: (windowId, hidden) => page.setWindowHidden(account.id, windowId, hidden)
                onRemoveConfirmed: page.remove(account.id)
                onExpandToggled: page.toggleExpanded(account.id)
                onDragMoved: offset => page.dragMoved(index, offset)
                onDragFinished: page.dragFinished(index)
            }
        }
    }

    SettingsGroup {
        title: page.tr("Add Account")
        description: page.tr("Sign in to another account without touching the one your CLI uses.")

        Repeater {
            model: Commands.ADDABLE

            AddAccountRow {
                required property string modelData
                required property int index

                provider: modelData
                lang: page.lang
                separated: index > 0
                launched: page.launchedProvider === modelData
                onSignInRequested: (provider, label) => page.signIn(provider, label)
            }
        }
    }

    CommandRunner {
        id: runner

        onExited: (command, exitCode, stdout) => page.commandExited(command, exitCode, stdout)
    }
}
