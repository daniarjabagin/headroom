pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import "logic/I18n.js" as I18n

QQC2.Menu {
    id: menu

    required property string providerName
    required property var links
    required property bool starred
    required property bool canStar
    required property bool canShare
    required property string lang

    signal refreshRequested
    signal hideRequested
    signal starToggled
    signal linkOpened(string url)
    signal shareRequested
    signal copyRequested

    function tr(msgid, values) {
        return I18n.tr(lang, msgid, values);
    }

    QQC2.MenuItem {
        objectName: "menuRefresh"
        icon.name: "view-refresh"
        text: menu.tr("Refresh {provider}", {
            provider: menu.providerName
        })
        onTriggered: menu.refreshRequested()
    }

    QQC2.MenuItem {
        objectName: "menuHide"
        icon.name: "view-hidden"
        text: menu.tr("Hide from popup")
        onTriggered: menu.hideRequested()
    }

    QQC2.MenuItem {
        objectName: "menuStar"
        visible: menu.canStar
        height: visible ? implicitHeight : 0
        checkable: true
        checked: menu.starred
        text: menu.tr("Always show")
        onTriggered: {
            menu.starToggled();
            checked = Qt.binding(() => menu.starred);
        }
    }

    QQC2.MenuSeparator {
        visible: menu.links.length > 0
        height: visible ? implicitHeight : 0
    }

    Instantiator {
        model: menu.links

        delegate: QQC2.MenuItem {
            required property var modelData

            icon.name: modelData.icon
            text: `${modelData.label} · ${modelData.host}`
            onTriggered: menu.linkOpened(modelData.url)
        }

        onObjectAdded: (index, object) => menu.insertItem(4 + index, object)
        onObjectRemoved: (index, object) => menu.removeItem(object)
    }

    QQC2.MenuSeparator {}

    QQC2.MenuItem {
        objectName: "menuShare"
        enabled: menu.canShare
        icon.name: "document-share"
        text: menu.tr("Share as image…")
        onTriggered: menu.shareRequested()
    }

    QQC2.MenuItem {
        objectName: "menuCopy"
        enabled: menu.canShare
        icon.name: "edit-copy"
        text: menu.tr("Copy as text")
        onTriggered: menu.copyRequested()
    }
}
