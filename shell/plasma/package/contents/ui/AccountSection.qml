pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Account.js" as Account
import "logic/Format.js" as Format
import "logic/Metrics.js" as Metrics

ColumnLayout {
    id: section

    required property var account
    required property bool showName
    required property bool offline
    required property var now
    required property bool alwaysShowPacing
    required property bool expanded
    readonly property var notices: Account.notices(account, offline)
    readonly property bool showsQuotas: Account.showsQuotas(account)

    signal refreshRequested(string accountId)
    signal copyRequested(string text)
    signal expandToggled(string accountId)

    function runAction(kind, value) {
        if (kind === "copy")
            copyRequested(value);
        else
            refreshRequested(value);
    }

    Layout.fillWidth: true
    spacing: Kirigami.Units.smallSpacing

    AccountHeader {
        account: section.account
        showName: section.showName
        offline: section.offline
        now: section.now
    }

    Card {
        Repeater {
            model: section.notices.length

            NoticeRow {
                required property int index

                entry: section.notices[index]
                onActionTriggered: (kind, value) => section.runAction(kind, value)
            }
        }

        Repeater {
            model: section.showsQuotas ? section.account.windows.length : 0

            QuotaRow {
                required property int index

                window: section.account.windows[index]
                now: section.now
                alwaysShowPacing: section.alwaysShowPacing
            }
        }

        Loader {
            Layout.fillWidth: true
            active: section.showsQuotas && section.account.usage !== null
            visible: active

            sourceComponent: UsageTrend {
                usage: section.account.usage
            }
        }

        Caret {
            visible: section.showsQuotas && Account.hasExtras(section.account)
            expanded: section.expanded
            onClicked: section.expandToggled(section.account.id)
        }

        ColumnLayout {
            visible: section.showsQuotas && section.expanded
            Layout.fillWidth: true
            Layout.topMargin: Metrics.textRowPadding(Kirigami.Units) - Kirigami.Units.smallSpacing / 2
            Layout.bottomMargin: Metrics.textRowPadding(Kirigami.Units)
            spacing: 0

            Repeater {
                model: Account.spendRows(section.account.usage)

                ValueRow {
                    required property var modelData

                    title: modelData.title
                    value: Format.spendLine(modelData.totals)
                    tooltip: Format.spendTooltip(modelData.totals)
                }
            }

            Repeater {
                model: section.account.balances

                ValueRow {
                    required property var modelData

                    title: modelData.label
                    value: Format.balanceValue(modelData)
                }
            }
        }
    }
}
