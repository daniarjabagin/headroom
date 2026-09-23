pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Account.js" as Account
import "logic/Metrics.js" as Metrics
import "logic/State.js" as State

Item {
    id: section

    required property var account
    required property var providers
    required property bool showName
    required property bool offline
    required property var now
    required property bool live
    required property var display
    required property string lang
    required property real appear
    required property bool expanded
    required property bool canReorder
    required property bool lifted
    required property real dragOffset
    required property string indicator
    required property real gap
    readonly property var notices: Account.notices(lang, account, offline, providers)
    readonly property var windows: Account.showsQuotas(account) ? State.shownWindows(account) : []
    readonly property bool extrasOpen: Account.extrasAlwaysOpen(account)

    signal refreshRequested(string accountId)
    signal signInRequested(string providerId)
    signal expandToggled(string accountId)
    signal dragMoved(real offset)
    signal dragFinished
    signal valueModeToggled
    signal resetFormatToggled

    function runAction(kind, value) {
        if (kind === "signin")
            signInRequested(value);
        else
            refreshRequested(value);
    }

    Layout.fillWidth: true
    implicitHeight: column.implicitHeight
    z: lifted ? 2 : 0
    opacity: appear * (lifted ? 0.9 : 1)

    transform: Translate {
        y: section.dragOffset + (1 - section.appear) * Kirigami.Units.gridUnit * 0.75
    }

    ColumnLayout {
        id: column

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        spacing: Kirigami.Units.smallSpacing

        AccountHeader {
            account: section.account
            showName: section.showName
            offline: section.offline
            now: section.now
            lang: section.lang
            canReorder: section.canReorder
            onDragMoved: offset => section.dragMoved(offset)
            onDragFinished: section.dragFinished()
        }

        Card {
            visible: section.notices.length > 0 || section.windows.length > 0 || trend.active || section.extrasOpen
            hoverable: true
            lifted: section.lifted

            Repeater {
                model: section.notices.length

                NoticeRow {
                    required property int index

                    entry: section.notices[index]
                    onActionTriggered: (kind, value) => section.runAction(kind, value)
                }
            }

            Repeater {
                model: section.windows.length

                QuotaRow {
                    required property int index

                    window: section.windows[index]
                    now: section.now
                    live: section.live
                    display: section.display
                    lang: section.lang
                    appear: section.appear
                    onValueModeToggled: section.valueModeToggled()
                    onResetFormatToggled: section.resetFormatToggled()
                }
            }

            Loader {
                id: trend

                Layout.fillWidth: true
                active: Account.showsTrend(section.account, section.display)
                visible: active

                sourceComponent: UsageTrend {
                    usage: section.account.usage
                    lang: section.lang
                }
            }

            Caret {
                visible: Account.hasExtras(section.account, section.display) && !section.extrasOpen
                expanded: section.expanded
                onClicked: section.expandToggled(section.account.id)
            }

            AccountExtras {
                account: section.account
                display: section.display
                lang: section.lang
                expanded: section.extrasOpen || (section.expanded && Account.hasExtras(section.account, section.display))
            }
        }
    }

    Rectangle {
        visible: section.indicator !== ""
        x: Metrics.headerInset(Kirigami.Units)
        width: parent.width - x * 2
        height: Metrics.hairline(Kirigami.Units) * 2
        radius: height / 2
        y: section.indicator === "below" ? section.height + section.gap / 2 - height / 2 : -section.gap / 2 - height / 2
        color: Kirigami.Theme.highlightColor
    }
}
