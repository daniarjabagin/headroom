pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/AccountList.js" as AccountList
import "logic/I18n.js" as I18n
import "logic/Preferences.js" as Preferences
import "logic/Providers.js" as Providers
import "logic/Settings.js" as Settings
import "logic/Tokens.js" as Tokens

ColumnLayout {
    id: detail

    required property var account
    required property var display
    required property var links
    required property string lang
    required property int position
    required property int count
    property bool canStar: true
    property bool animated: true
    readonly property var status: AccountList.status(lang, account)
    readonly property string subtitle: [account.providerName, AccountList.subtitle(lang, account)].filter(part => part !== "").join(" · ")
    readonly property bool starred: Settings.isStarred(display, account.id)
    readonly property real fieldWidth: Kirigami.Units.gridUnit * 8
    readonly property string accountId: account.id

    signal hiddenToggled(bool hidden)
    signal starToggled(bool starred)
    signal labelApplied(string label)
    signal moveRequested(int offset)
    signal windowToggled(string windowId, bool hidden)
    signal signInRequested(string accountId)
    signal linkOpened(string url)
    signal removeConfirmed

    function tr(msgid, values) {
        return I18n.tr(lang, msgid, values);
    }

    objectName: "accountDetail"
    onAccountIdChanged: labelField.text = account.label ?? ""
    Layout.fillWidth: true
    spacing: Kirigami.Units.gridUnit

    Card {
        verticalPadding: Kirigami.Units.smallSpacing

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.largeSpacing

            ProviderIcon {
                provider: detail.account.provider
                implicitWidth: Kirigami.Units.iconSizes.medium
                implicitHeight: implicitWidth
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                TextLabel {
                    objectName: "detailTitle"
                    Layout.fillWidth: true
                    role: "title"
                    text: Providers.accountName(detail.account)
                    elide: Text.ElideRight
                }

                TextLabel {
                    Layout.fillWidth: true
                    role: "caption"
                    emphasis: "secondary"
                    text: detail.subtitle
                    elide: Text.ElideRight
                }

                TextLabel {
                    visible: detail.status.text !== ""
                    Layout.fillWidth: true
                    role: "caption"
                    color: Tokens.noticeColor(Kirigami.Theme, detail.status.kind)
                    text: detail.status.text
                    elide: Text.ElideRight
                }
            }
        }

        SettingsRow {
            title: detail.tr("Show in the panel and popup")
            subtitle: detail.tr("Hidden accounts keep updating but leave the panel and notifications.")

            QQC2.Switch {
                objectName: "accountVisible"
                checked: !detail.account.hidden
                onToggled: detail.hiddenToggled(!checked)
            }
        }

        SettingsRow {
            visible: detail.canStar
            title: detail.tr("Card in the popup")
            subtitle: Preferences.starLabel(detail.lang, detail.starred)

            IconButton {
                objectName: "accountStar"
                iconName: detail.starred ? "starred-symbolic" : "non-starred-symbolic"
                text: detail.starred ? detail.tr("Unstar") : detail.tr("Star")
                animated: detail.animated
                onClicked: detail.starToggled(!detail.starred)
            }
        }

        SettingsRow {
            title: detail.tr("Label")
            subtitle: detail.tr("Shown instead of the email")

            QQC2.TextField {
                id: labelField

                objectName: "labelField"
                Layout.preferredWidth: detail.fieldWidth
                text: detail.account.label ?? ""
                placeholderText: detail.account.email ?? detail.tr("Label")
                maximumLength: 64
                onAccepted: detail.labelApplied(text.trim())
            }

            SmallButton {
                text: detail.tr("Apply")
                enabled: labelField.text.trim() !== (detail.account.label ?? "")
                onClicked: detail.labelApplied(labelField.text.trim())
            }
        }

        SettingsRow {
            visible: detail.count > 1
            title: detail.tr("Position in the popup")
            subtitle: detail.tr("{position} of {count}", {
                position: detail.position + 1,
                count: detail.count
            })

            IconButton {
                objectName: "moveUp"
                iconName: "go-up"
                text: detail.tr("Move up")
                enabled: detail.position > 0
                opacity: enabled ? 1 : 0.4
                animated: detail.animated
                onClicked: detail.moveRequested(-1)
            }

            IconButton {
                objectName: "moveDown"
                iconName: "go-down"
                text: detail.tr("Move down")
                enabled: detail.position < detail.count - 1
                opacity: enabled ? 1 : 0.4
                animated: detail.animated
                onClicked: detail.moveRequested(1)
            }
        }
    }

    AccountLimitsGroup {
        account: detail.account
        display: detail.display
        lang: detail.lang
        onWindowToggled: (windowId, hidden) => detail.windowToggled(windowId, hidden)
    }

    AccountActionsGroup {
        account: detail.account
        links: detail.links
        lang: detail.lang
        animated: detail.animated
        onSignInRequested: accountId => detail.signInRequested(accountId)
        onLinkOpened: url => detail.linkOpened(url)
        onRemoveConfirmed: detail.removeConfirmed()
    }
}
