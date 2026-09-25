pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
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
    property string selectedProvider: ""
    readonly property var chosenProvider: Registry.findProvider(providers, selectedProvider) ?? providers[0] ?? null
    property var expandedIds: []
    property string launchedProvider: ""
    property var pendingKinds: ({})
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

    SettingsGroup {
        title: page.tr("Accounts")
        description: page.tr("Drag to reorder. Hidden accounts keep updating but leave the panel and notifications.")

        SettingsRow {
            visible: page.accounts.length === 0
            separated: false
            title: page.tr("No accounts yet")
            subtitle: page.tr("Sign in with a supported CLI, or add an account below.")
        }

        Repeater {
            id: rows

            model: page.accounts.length

            AccountConfigRow {
                required property int index

                account: page.accounts[index]
                display: page.current.display
                lang: page.lang
                animated: Motion.enabled(Kirigami.Units, page.current.reducedMotion)
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
        description: page.tr("Pick a service. Accounts added here never touch the one your CLI uses.")

        SettingsRow {
            visible: page.chosenProvider === null
            separated: false
            title: page.daemon.providersRequested ? page.tr("No providers available") : page.tr("Loading…")
        }

        ProviderPicker {
            visible: page.chosenProvider !== null
            providers: page.providers
            selected: page.chosenProvider?.id ?? ""
            animated: Motion.enabled(Kirigami.Units, page.current.reducedMotion)
            onPicked: providerId => {
                page.selectedProvider = providerId;
                page.launchedProvider = "";
            }
        }

        Loader {
            Layout.fillWidth: true
            active: page.chosenProvider !== null

            sourceComponent: AddAccountRow {
                provider: page.chosenProvider
                lang: page.lang
                launched: page.launchedProvider === page.chosenProvider.id
                onAddRequested: label => page.add(page.chosenProvider, label)
            }
        }
    }

    CommandRunner {
        id: runner

        onExited: (command, exitCode, stdout) => page.commandExited(command, exitCode, stdout)
    }
}
