pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/AccountList.js" as AccountList
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Settings.js" as Settings
import "logic/Tokens.js" as Tokens

Card {
    id: pane

    required property var accounts
    required property var display
    required property string lang
    required property string selectedId
    required property bool adding
    readonly property var rows: AccountList.rows(accounts)

    signal accountSelected(string accountId)
    signal addRequested

    objectName: "accountList"
    verticalPadding: Kirigami.Units.smallSpacing
    spacing: Math.round(Kirigami.Units.smallSpacing / 2)

    TextLabel {
        visible: pane.accounts.length === 0
        Layout.fillWidth: true
        Layout.margins: Metrics.rowInset(Kirigami.Units)
        emphasis: "secondary"
        wrapMode: Text.Wrap
        text: I18n.tr(pane.lang, "No accounts yet")
    }

    Repeater {
        model: pane.rows.length

        Loader {
            id: entry

            required property int index
            readonly property var row: pane.rows[index]

            Layout.fillWidth: true
            sourceComponent: row.kind === "provider" ? providerHeading : accountRow

            Component {
                id: providerHeading

                TextLabel {
                    topPadding: entry.index > 0 ? Kirigami.Units.mediumSpacing : Kirigami.Units.smallSpacing
                    bottomPadding: Math.round(Kirigami.Units.smallSpacing / 2)
                    leftPadding: Metrics.rowInset(Kirigami.Units)
                    role: "caption"
                    weight: Font.DemiBold
                    emphasis: "secondary"
                    text: entry.row.title
                }
            }

            Component {
                id: accountRow

                AccountListRow {
                    objectName: "accountListRow"
                    account: entry.row.account
                    lang: pane.lang
                    current: !pane.adding && entry.row.key === pane.selectedId
                    starred: Settings.isStarred(pane.display, entry.row.key)
                    animated: pane.animated
                    onClicked: pane.accountSelected(entry.row.key)
                }
            }
        }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.topMargin: Kirigami.Units.smallSpacing
        Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
        Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
        implicitHeight: Metrics.hairline(Kirigami.Units)
        color: Tokens.separator(Kirigami.Theme)
    }

    SmallButton {
        objectName: "addAccountButton"
        Layout.margins: Kirigami.Units.largeSpacing
        Layout.alignment: Qt.AlignLeft
        primary: pane.adding
        text: I18n.tr(pane.lang, "Add Account…")
        onClicked: pane.addRequested()
    }
}
