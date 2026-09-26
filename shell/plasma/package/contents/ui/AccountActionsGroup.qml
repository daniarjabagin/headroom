pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Account.js" as Account
import "logic/AccountList.js" as AccountList
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Providers.js" as Providers
import "logic/QuickLinks.js" as QuickLinks
import "logic/Tokens.js" as Tokens

SettingsGroup {
    id: group

    required property var account
    required property var links
    required property string lang
    property bool animated: true
    property bool confirming: false
    readonly property string signInTarget: AccountList.signInTarget(account) ?? ""
    readonly property var linkEntries: QuickLinks.menuEntries(lang, links)
    readonly property var removal: Account.removal(lang, account)
    readonly property string accountId: account.id

    signal signInRequested(string accountId)
    signal linkOpened(string url)
    signal removeConfirmed

    function tr(msgid, values) {
        return I18n.tr(lang, msgid, values);
    }

    onAccountIdChanged: confirming = false

    SettingsRow {
        objectName: "signInAgainRow"
        visible: group.signInTarget !== ""
        separated: false
        title: group.tr("Sign in again")
        subtitle: group.tr("Opens a terminal to renew the sign-in for this account")

        SmallButton {
            objectName: "signInAgain"
            primary: true
            animated: group.animated
            text: group.tr("Sign in again…")
            onClicked: group.signInRequested(group.signInTarget)
        }
    }

    ColumnLayout {
        objectName: "linksRow"
        visible: group.linkEntries.length > 0
        Layout.fillWidth: true
        Layout.margins: Metrics.rowInset(Kirigami.Units)
        spacing: Kirigami.Units.smallSpacing

        Rectangle {
            visible: group.signInTarget !== ""
            Layout.fillWidth: true
            Layout.bottomMargin: Kirigami.Units.largeSpacing
            implicitHeight: Metrics.hairline(Kirigami.Units)
            color: Tokens.separator(Kirigami.Theme)
        }

        TextLabel {
            role: "label"
            weight: Font.Medium
            text: group.tr("Links")
        }

        Flow {
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            Repeater {
                model: group.linkEntries

                LinkText {
                    required property var modelData

                    text: `${modelData.label} ↗`
                    onClicked: group.linkOpened(modelData.url)

                    HoverTip {
                        text: parent.modelData.host
                    }
                }
            }
        }
    }

    SettingsRow {
        visible: !group.confirming
        separated: group.signInTarget !== "" || group.linkEntries.length > 0
        title: group.tr("Remove from Headroom")
        subtitle: group.removal.subtitle

        SmallButton {
            objectName: "removeAccount"
            animated: group.animated
            text: group.tr("Remove")
            onClicked: group.confirming = true
        }
    }

    NoticeRow {
        visible: group.confirming
        Layout.topMargin: Kirigami.Units.largeSpacing
        Layout.bottomMargin: Kirigami.Units.largeSpacing
        entry: ({
                kind: "error",
                title: group.tr("Remove {name}?", {
                    name: Providers.accountName(group.account)
                }),
                detail: group.removal.confirmation,
                note: "",
                actions: [
                    {
                        kind: "cancel",
                        label: group.tr("Cancel"),
                        value: ""
                    },
                    {
                        kind: "remove",
                        label: group.tr("Remove"),
                        value: group.account.id
                    }
                ]
            })
        animated: group.animated
        onActionTriggered: kind => {
            group.confirming = false;
            if (kind === "remove")
                group.removeConfirmed();
        }
    }
}
