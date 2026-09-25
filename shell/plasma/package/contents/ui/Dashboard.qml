pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Collapse.js" as Collapse
import "logic/Combined.js" as Combined
import "logic/Density.js" as Density
import "logic/Incident.js" as Incident
import "logic/Motion.js" as Motion
import "logic/Order.js" as Order
import "logic/ProviderStatus.js" as ProviderStatus
import "logic/Registry.js" as Registry
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
    required property bool reducedMotion
    readonly property bool capable: snapshot.supports06
    readonly property var accounts: State.visibleAccounts(snapshot)
    readonly property var parts: Collapse.partition(Combined.cards(snapshot, accounts))
    readonly property var cards: parts.shown
    readonly property var folded: parts.folded
    readonly property bool showSpend: display.showSpend && snapshot.spend !== null
    readonly property int spendOffset: showSpend ? 1 : 0
    readonly property int itemCount: cards.length + spendOffset
    readonly property bool motion: Motion.enabled(Kirigami.Units, reducedMotion)
    property string localPeriod: "today"
    property string localUnit: "cost"
    property string localBreakdown: "models"
    readonly property string period: capable ? display.spendPeriod : localPeriod
    readonly property string unit: capable ? display.spendUnit : localUnit
    readonly property string breakdown: capable ? display.spendBreakdown : localBreakdown
    property var expandedIds: []
    property bool foldedOpen: false
    property real foldReveal: 0
    property int dragIndex: -1
    property real dragOffset: 0
    property int dropTarget: -1
    readonly property var dropSlot: Order.indicatorSlot(dragIndex, dropTarget, cards.length)

    signal refreshRequested(string accountId)
    signal signInRequested(string providerId)
    signal settingsRequested
    signal orderRequested(var ids)
    signal displayPatched(var patch)
    signal hideRequested(var accountIds)
    signal linkRequested(string url)
    signal shareRequested(var card, string plan)
    signal copyRequested(var card, string plan)

    function toggleExpanded(accountId) {
        expandedIds = expandedIds.includes(accountId) ? expandedIds.filter(id => id !== accountId) : expandedIds.concat([accountId]);
    }

    function appear(index) {
        return Motion.easeOutCubic(Motion.stagger(reveal, index, itemCount));
    }

    function foldAppear(index) {
        return Motion.easeOutCubic(Motion.stagger(foldReveal, index, folded.length));
    }

    function setFolded(open) {
        foldAnimation.stop();
        if (open)
            foldedOpen = true;
        if (!motion) {
            foldReveal = open ? 1 : 0;
            foldedOpen = open;
            return;
        }
        foldAnimation.to = open ? 1 : 0;
        foldAnimation.duration = open ? Kirigami.Units.longDuration : Kirigami.Units.shortDuration;
        foldAnimation.start();
    }

    function pick(key, localKey, value) {
        if (capable)
            displayPatched({
                [key]: value
            });
        else
            dashboard[localKey] = value;
    }

    function cardPlan(card) {
        return (card.kind === "account" ? card.account.plan : Combined.headerAccount(lang, card).plan) ?? "";
    }

    function isStarred(card) {
        return card.accountIds.some(id => Settings.isStarred(display, id));
    }

    function toggleStar(card) {
        const starred = isStarred(card);
        const next = card.accountIds.reduce((current, id) => Object.assign({}, current, {
                starredAccounts: Settings.starredPatch(current, id, !starred).starredAccounts
            }), display);
        displayPatched({
            starredAccounts: next.starredAccounts
        });
    }

    function refreshCard(card) {
        card.accountIds.forEach(id => refreshRequested(id));
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
        orderRequested(Order.reorderedGroups(all, cards.map(card => card.accountIds), index, target));
    }

    component CardSection: AccountSection {
        id: cardSection

        required property var card

        account: card.account ?? Combined.headerAccount(dashboard.lang, card)
        group: card.group
        members: card.members
        providers: dashboard.providers
        showName: card.kind === "account" && State.showsName(account, dashboard.accounts)
        offline: dashboard.snapshot.offline
        now: dashboard.now
        live: dashboard.live
        display: dashboard.display
        lang: dashboard.lang
        expanded: dashboard.expandedIds.includes(account.id)
        gap: dashboard.spacing
        reducedMotion: dashboard.reducedMotion
        links: Registry.providerLinks(dashboard.providers, account.provider)
        incident: Incident.notice(dashboard.lang, ProviderStatus.forProvider(dashboard.snapshot.providerStatus, account.provider), dashboard.now)
        starred: dashboard.isStarred(card)
        canStar: dashboard.capable
        onRefreshRequested: accountId => dashboard.refreshRequested(accountId)
        onMenuRefreshRequested: dashboard.refreshCard(cardSection.card)
        onSignInRequested: providerId => dashboard.signInRequested(providerId)
        onSettingsRequested: dashboard.settingsRequested()
        onExpandToggled: accountId => dashboard.toggleExpanded(accountId)
        onValueModeToggled: dashboard.displayPatched(Settings.toggledValueMode(dashboard.display))
        onResetFormatToggled: dashboard.displayPatched(Settings.toggledResetFormat(dashboard.display))
        onHideRequested: dashboard.hideRequested(cardSection.card.accountIds)
        onStarToggled: dashboard.toggleStar(cardSection.card)
        onLinkOpened: url => dashboard.linkRequested(url)
        onShareRequested: dashboard.shareRequested(cardSection.card, dashboard.cardPlan(cardSection.card))
        onCopyRequested: dashboard.copyRequested(cardSection.card, dashboard.cardPlan(cardSection.card))
    }

    spacing: Density.sectionGap(Kirigami.Units, Density.isCompact(display))
    onFoldedChanged: {
        if (folded.length === 0) {
            foldedOpen = false;
            foldReveal = 0;
        }
    }

    Loader {
        Layout.fillWidth: true
        active: dashboard.showSpend
        visible: active

        sourceComponent: SpendCard {
            spend: dashboard.snapshot.spend
            period: dashboard.period
            unit: dashboard.unit
            breakdown: dashboard.breakdown
            capable: dashboard.capable
            compact: Density.isCompact(dashboard.display)
            lang: dashboard.lang
            appear: dashboard.appear(0)
            onPeriodSelected: key => dashboard.pick("spendPeriod", "localPeriod", key)
            onUnitSelected: key => dashboard.pick("spendUnit", "localUnit", key)
            onBreakdownSelected: key => dashboard.pick("spendBreakdown", "localBreakdown", key)
        }
    }

    Repeater {
        id: sections

        model: dashboard.cards.length

        CardSection {
            required property int index

            card: dashboard.cards[index]
            appear: dashboard.appear(index + dashboard.spendOffset)
            canReorder: dashboard.cards.length > 1
            lifted: dashboard.dragIndex === index
            dragOffset: lifted ? dashboard.dragOffset : 0
            indicator: dashboard.dropSlot?.index === index ? (dashboard.dropSlot.below ? "below" : "above") : ""
            onDragMoved: offset => dashboard.dragMoved(index, offset)
            onDragFinished: dashboard.dragFinished(index)
        }
    }

    CollapsedRow {
        visible: dashboard.folded.length > 0 && !dashboard.foldedOpen
        folded: dashboard.folded
        lang: dashboard.lang
        opacity: dashboard.appear(dashboard.itemCount - 1)
        onClicked: dashboard.setFolded(true)
    }

    FoldDivider {
        visible: dashboard.foldedOpen
        Layout.bottomMargin: -Math.round(Kirigami.Units.smallSpacing * 1.5)
        lang: dashboard.lang
        opacity: dashboard.foldReveal
        onCollapseRequested: dashboard.setFolded(false)
    }

    Repeater {
        model: dashboard.foldedOpen ? dashboard.folded.length : 0

        CardSection {
            required property int index

            card: dashboard.folded[index]
            appear: dashboard.foldAppear(index)
            canReorder: false
            lifted: false
            dragOffset: 0
            indicator: ""
        }
    }

    NumberAnimation {
        id: foldAnimation

        target: dashboard
        property: "foldReveal"
        easing.type: Easing.OutCubic
        onFinished: {
            if (dashboard.foldReveal === 0)
                dashboard.foldedOpen = false;
        }
    }
}
