pragma ComponentBehavior: Bound

import QtQuick
import org.kde.plasma.components as PlasmaComponents3
import "logic/I18n.js" as I18n
import "logic/Providers.js" as Providers

PlasmaComponents3.Menu {
    id: optionsRoot

    property var accounts: []
    property string lang: "en"

    signal refreshRequested
    signal hiddenRequested(string accountId, bool hidden)
    signal settingsRequested

    PlasmaComponents3.MenuItem {
        text: I18n.tr(optionsRoot.lang, "Refresh now")
        icon.name: "view-refresh"
        onTriggered: optionsRoot.refreshRequested()
    }

    PlasmaComponents3.Menu {
        id: accountsMenu

        title: I18n.tr(optionsRoot.lang, "Hide accounts…")
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
        text: I18n.tr(optionsRoot.lang, "Settings…")
        icon.name: "configure"
        onTriggered: optionsRoot.settingsRequested()
    }
}
