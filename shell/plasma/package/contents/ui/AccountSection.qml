pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Account.js" as Account
import "logic/Density.js" as Density
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
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
    required property bool reducedMotion
    property var group: null
    property var members: []
    property var links: ({})
    property var incident: null
    property bool starred: false
    property bool canStar: false
    readonly property bool compact: Density.isCompact(display)
    readonly property bool animated: Motion.enabled(Kirigami.Units, reducedMotion)
    readonly property var notices: Account.notices(lang, account, offline, providers)
    readonly property var plates: Account.plates(notices)
    readonly property var infoLines: Account.infoLines(notices)
    readonly property var windows: group !== null ? group.windows : Account.showsQuotas(account) ? State.shownWindows(account) : []
    readonly property bool extrasOpen: Account.extrasAlwaysOpen(account)

    signal refreshRequested(string accountId)
    signal signInRequested(string providerId)
    signal settingsRequested
    signal expandToggled(string accountId)
    signal dragMoved(real offset)
    signal dragFinished
    signal valueModeToggled
    signal resetFormatToggled
    signal menuRefreshRequested
    signal hideRequested
    signal starToggled
    signal linkOpened(string url)
    signal shareRequested
    signal copyRequested

    function runAction(kind, value) {
        if (kind === "signin")
            signInRequested(value);
        else if (kind === "settings")
            settingsRequested();
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
        spacing: Density.headerGap(Kirigami.Units, section.compact)

        AccountHeader {
            account: section.account
            showName: section.showName
            offline: section.offline
            now: section.now
            lang: section.lang
            canReorder: section.canReorder
            links: section.links
            incident: section.incident
            compact: section.compact
            starred: section.starred
            canStar: section.canStar
            canShare: section.windows.length > 0
            animated: section.animated
            onDragMoved: offset => section.dragMoved(offset)
            onDragFinished: section.dragFinished()
            onRefreshRequested: section.menuRefreshRequested()
            onHideRequested: section.hideRequested()
            onStarToggled: section.starToggled()
            onLinkOpened: url => section.linkOpened(url)
            onShareRequested: section.shareRequested()
            onCopyRequested: section.copyRequested()
        }

        Card {
            visible: section.notices.length > 0 || section.incident !== null || section.windows.length > 0 || trend.active || section.extrasOpen
            hoverable: true
            animated: section.animated
            lifted: section.lifted
            verticalPadding: Density.cardGutter(Kirigami.Units, section.compact)

            Repeater {
                model: section.plates.length

                NoticeRow {
                    required property int index

                    entry: section.plates[index]
                    animated: section.animated
                    onActionTriggered: (kind, value) => section.runAction(kind, value)
                }
            }

            Loader {
                Layout.fillWidth: true
                active: section.incident !== null
                visible: active

                sourceComponent: StatusNotice {
                    notice: section.incident
                    lang: section.lang
                    compact: section.compact
                    animated: section.animated
                    onLinkActivated: url => section.linkOpened(url)
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
                    members: section.members
                    animated: section.animated
                    onValueModeToggled: section.valueModeToggled()
                    onResetFormatToggled: section.resetFormatToggled()
                }
            }

            Repeater {
                model: section.infoLines.length

                NoticeLine {
                    required property int index

                    text: section.infoLines[index].title
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
                    stripHeight: Density.trendHeight(Kirigami.Units, section.compact)
                    animated: section.animated
                }
            }

            Caret {
                visible: Account.hasExtras(section.account, section.display) && !section.extrasOpen
                topPadding: Density.cardGutter(Kirigami.Units, section.compact)
                bottomPadding: Density.cardGutter(Kirigami.Units, section.compact)
                expanded: section.expanded
                animated: section.animated
                onClicked: section.expandToggled(section.account.id)
            }

            AccountExtras {
                account: section.account
                display: section.display
                lang: section.lang
                animated: section.animated
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
