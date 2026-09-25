pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import "logic/I18n.js" as I18n

FittedMenu {
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

    MenuEntry {
        objectName: "menuRefresh"
        leadIcon: "view-refresh"
        text: menu.tr("Refresh {provider}", {
            provider: menu.providerName
        })
        onTriggered: menu.refreshRequested()
    }

    MenuEntry {
        objectName: "menuHide"
        leadIcon: "view-hidden"
        text: menu.tr("Hide from popup")
        onTriggered: menu.hideRequested()
    }

    MenuEntry {
        objectName: "menuStar"
        visible: menu.canStar
        height: visible ? implicitHeight : 0
        leadIcon: menu.starred ? "starred-symbolic" : "non-starred-symbolic"
        text: menu.tr("Always show")
        onTriggered: menu.starToggled()
    }

    QQC2.MenuSeparator {
        visible: menu.links.length > 0
        height: visible ? implicitHeight : 0
    }

    Instantiator {
        model: menu.links

        delegate: MenuEntry {
            required property var modelData

            leadIcon: modelData.icon
            text: modelData.label
            detail: modelData.menuHost
            onTriggered: menu.linkOpened(modelData.url)
        }

        onObjectAdded: (index, object) => menu.insertItem(4 + index, object)
        onObjectRemoved: (index, object) => menu.removeItem(object)
    }

    QQC2.MenuSeparator {}

    MenuEntry {
        objectName: "menuShare"
        enabled: menu.canShare
        leadIcon: "document-share"
        text: menu.tr("Share as image…")
        onTriggered: menu.shareRequested()
    }

    MenuEntry {
        objectName: "menuCopy"
        enabled: menu.canShare
        leadIcon: "edit-copy"
        text: menu.tr("Copy as text")
        onTriggered: menu.copyRequested()
    }
}
