pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasmoid
import "logic/I18n.js" as I18n
import "logic/Settings.js" as Settings
import "logic/State.js" as State
import "logic/Summary.js" as Summary

PlasmoidItem {
    id: root

    readonly property int liveTickMs: 1000
    readonly property int activeTickMs: 30000
    readonly property int idleTickMs: 60000
    property var now: new Date()
    readonly property var view: daemon.view
    readonly property bool ready: view.kind === "ready"
    readonly property var headline: ready ? view.state.headline : null
    property var lastDisplay: Settings.parseDisplay(null)
    readonly property var display: ready ? view.state.display : lastDisplay
    readonly property string lang: I18n.resolve(display.language, Qt.locale().name)
    readonly property bool live: expanded && ready && State.needsLiveClock(view.state, now)
    readonly property var tip: Summary.tooltip(lang, view, now)
    readonly property var systemTheme: Kirigami.Theme
    readonly property bool reducedMotion: daemon.settings?.reducedMotion ?? false

    toolTipMainText: tip.main
    toolTipSubText: tip.sub
    onDisplayChanged: {
        if (ready)
            lastDisplay = display;
    }
    onExpandedChanged: {
        now = new Date();
        if (expanded)
            daemon.refresh("");
    }

    compactRepresentation: CompactRepresentation {
        headline: root.headline
        display: root.display
        lang: root.lang
        reducedMotion: root.reducedMotion
        stale: root.ready && root.headline !== null && State.isHeadlineStale(root.view.state)
        vertical: Plasmoid.formFactor === PlasmaCore.Types.Vertical
        expanded: root.expanded
        onActivated: wasExpanded => root.expanded = !wasExpanded
    }

    fullRepresentation: FullRepresentation {
        view: root.view
        now: root.now
        live: root.live
        display: root.display
        lang: root.lang
        expanded: root.expanded
        systemTheme: root.systemTheme
        reducedMotion: root.reducedMotion
        versionText: `Headroom ${Plasmoid.metaData.version}`
        onRefreshRequested: accountId => daemon.refresh(accountId)
        onRefreshNowRequested: onFailed => daemon.refreshNow(onFailed)
        onOrderRequested: ids => daemon.setOrder(ids)
        onDisplayPatched: patch => daemon.patchDisplay(patch)
        onStartServiceRequested: daemon.startService()
        onSettingsRequested: Plasmoid.internalAction("configure").trigger()
    }

    DaemonClient {
        id: daemon

        active: root.expanded
        trackSettings: true
        onOpenRequested: root.expanded = true
    }

    Timer {
        interval: root.live ? root.liveTickMs : (root.expanded ? root.activeTickMs : root.idleTickMs)
        repeat: true
        running: true
        onTriggered: root.now = new Date()
    }

    Binding {
        target: Plasmoid
        property: "status"
        value: root.headline?.tone === "critical" ? PlasmaCore.Types.NeedsAttentionStatus : PlasmaCore.Types.ActiveStatus
    }
}
