pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/Order.js" as Order
import "logic/Settings.js" as Settings
import "logic/State.js" as State

ColumnLayout {
    id: dashboard

    required property var snapshot
    required property var providers
    required property var now
    required property bool live
    required property var display
    required property string lang
    required property real reveal
    readonly property var accounts: State.visibleAccounts(snapshot)
    readonly property bool showSpend: display.showSpend && snapshot.spend !== null
    readonly property int spendOffset: showSpend ? 1 : 0
    readonly property int itemCount: accounts.length + spendOffset
    property string period: "today"
    property var expandedIds: []
    property int dragIndex: -1
    property real dragOffset: 0
    property int dropTarget: -1
    readonly property var dropSlot: Order.indicatorSlot(dragIndex, dropTarget, accounts.length)

    signal refreshRequested(string accountId)
    signal signInRequested(string providerId)
    signal orderRequested(var ids)
    signal displayPatched(var patch)

    function toggleExpanded(accountId) {
        expandedIds = expandedIds.includes(accountId) ? expandedIds.filter(id => id !== accountId) : expandedIds.concat([accountId]);
    }

    function appear(index) {
        return Motion.easeOutCubic(Motion.stagger(reveal, index, itemCount));
    }

    function centers() {
        const found = [];
        for (let index = 0; index < sections.count; index++) {
            const item = sections.itemAt(index);
            found.push(item ? item.y + item.height / 2 : 0);
        }
        return found;
    }

    function dragMoved(index, offset) {
        const item = sections.itemAt(index);
        if (!item)
            return;
        dragIndex = index;
        dragOffset = offset;
        dropTarget = Order.targetIndex(centers(), index, item.y + item.height / 2 + offset);
    }

    function dragFinished(index) {
        const target = dropTarget;
        dragIndex = -1;
        dragOffset = 0;
        dropTarget = -1;
        if (target < 0 || target === index)
            return;
        const all = snapshot.accounts.map(account => account.id);
        orderRequested(Order.reordered(all, accounts.map(account => account.id), index, target));
    }

    spacing: Metrics.sectionGap(Kirigami.Units)

    Loader {
        Layout.fillWidth: true
        active: dashboard.showSpend
        visible: active

        sourceComponent: SpendCard {
            spend: dashboard.snapshot.spend
            period: dashboard.period
            lang: dashboard.lang
            appear: dashboard.appear(0)
            onPeriodSelected: key => dashboard.period = key
        }
    }

    Repeater {
        id: sections

        model: dashboard.accounts.length

        AccountSection {
            required property int index

            account: dashboard.accounts[index]
            providers: dashboard.providers
            showName: State.showsName(account, dashboard.accounts)
            offline: dashboard.snapshot.offline
            now: dashboard.now
            live: dashboard.live
            display: dashboard.display
            lang: dashboard.lang
            appear: dashboard.appear(index + dashboard.spendOffset)
            expanded: dashboard.expandedIds.includes(account.id)
            canReorder: dashboard.accounts.length > 1
            lifted: dashboard.dragIndex === index
            dragOffset: lifted ? dashboard.dragOffset : 0
            indicator: dashboard.dropSlot?.index === index ? (dashboard.dropSlot.below ? "below" : "above") : ""
            gap: dashboard.spacing
            onRefreshRequested: accountId => dashboard.refreshRequested(accountId)
            onSignInRequested: providerId => dashboard.signInRequested(providerId)
            onExpandToggled: accountId => dashboard.toggleExpanded(accountId)
            onDragMoved: offset => dashboard.dragMoved(index, offset)
            onDragFinished: dashboard.dragFinished(index)
            onValueModeToggled: dashboard.displayPatched(Settings.toggledValueMode(dashboard.display))
            onResetFormatToggled: dashboard.displayPatched(Settings.toggledResetFormat(dashboard.display))
        }
    }
}
