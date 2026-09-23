pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/State.js" as State

ColumnLayout {
    id: dashboard

    required property var snapshot
    required property var now
    required property bool alwaysShowPacing
    readonly property var accounts: State.visibleAccounts(snapshot)
    property string period: "today"
    property var expandedIds: []

    signal refreshRequested(string accountId)
    signal copyRequested(string text)

    function toggleExpanded(accountId) {
        expandedIds = expandedIds.includes(accountId) ? expandedIds.filter(id => id !== accountId) : expandedIds.concat([accountId]);
    }

    spacing: Metrics.sectionGap(Kirigami.Units)

    Loader {
        Layout.fillWidth: true
        active: dashboard.snapshot.spend !== null
        visible: active

        sourceComponent: SpendCard {
            spend: dashboard.snapshot.spend
            period: dashboard.period
            onPeriodSelected: key => dashboard.period = key
        }
    }

    Repeater {
        model: dashboard.accounts.length

        AccountSection {
            required property int index

            account: dashboard.accounts[index]
            showName: State.showsName(account, dashboard.accounts)
            offline: dashboard.snapshot.offline
            now: dashboard.now
            alwaysShowPacing: dashboard.alwaysShowPacing
            expanded: dashboard.expandedIds.includes(account.id)
            onRefreshRequested: accountId => dashboard.refreshRequested(accountId)
            onCopyRequested: text => dashboard.copyRequested(text)
            onExpandToggled: accountId => dashboard.toggleExpanded(accountId)
        }
    }
}
