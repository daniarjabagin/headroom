pragma ComponentBehavior: Bound

import QtQuick
import org.kde.plasma.components as PlasmaComponents3
import "logic/Providers.js" as Providers

PlasmaComponents3.Menu {
    id: optionsRoot

    property var accounts: []

    signal refreshRequested
    signal hiddenRequested(string accountId, bool hidden)
    signal settingsRequested

    PlasmaComponents3.MenuItem {
        text: "Refresh now"
        icon.name: "view-refresh"
        onTriggered: optionsRoot.refreshRequested()
    }

    PlasmaComponents3.Menu {
        id: accountsMenu

        title: "Hide accounts…"
        enabled: optionsRoot.accounts.length > 0

        Instantiator {
            model: optionsRoot.accounts.length
            onObjectAdded: (index, object) => accountsMenu.insertItem(index, object)
            onObjectRemoved: (index, object) => accountsMenu.removeItem(object)

            delegate: PlasmaComponents3.MenuItem {
                id: accountItem

                required property int index
                readonly property var account: optionsRoot.accounts[index]

                checkable: true
                checked: !accountItem.account.hidden
                text: Providers.accountTitle(accountItem.account, true)
                onTriggered: {
                    optionsRoot.hiddenRequested(accountItem.account.id, !accountItem.checked);
                    accountItem.checked = Qt.binding(() => !accountItem.account.hidden);
                }
            }
        }
    }

    PlasmaComponents3.MenuSeparator {}

    PlasmaComponents3.MenuItem {
        text: "Open settings…"
        icon.name: "configure"
        onTriggered: optionsRoot.settingsRequested()
    }
}
