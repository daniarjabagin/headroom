pragma ComponentBehavior: Bound

import QtQuick
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasmoid
import "logic/State.js" as State
import "logic/Summary.js" as Summary

PlasmoidItem {
    id: root

    readonly property int activeTickMs: 30000
    readonly property int idleTickMs: 60000
    property var now: new Date()
    readonly property var view: daemon.view
    readonly property bool ready: view.kind === "ready"
    readonly property var headline: ready ? view.state.headline : null
    readonly property var tip: Summary.tooltip(view, now)

    toolTipMainText: tip.main
    toolTipSubText: tip.sub
    onExpandedChanged: {
        now = new Date();
        if (expanded)
            daemon.refresh("");
    }

    compactRepresentation: CompactRepresentation {
        headline: root.headline
        stale: root.ready && root.headline !== null && State.isHeadlineStale(root.view.state)
        showPercentage: Plasmoid.configuration.showPercentage
        vertical: Plasmoid.formFactor === PlasmaCore.Types.Vertical
        expanded: root.expanded
        onActivated: wasExpanded => root.expanded = !wasExpanded
    }

    fullRepresentation: FullRepresentation {
        view: root.view
        now: root.now
        alwaysShowPacing: Plasmoid.configuration.alwaysShowPacing
        versionText: `Headroom ${Plasmoid.metaData.version}`
        onRefreshRequested: accountId => daemon.refresh(accountId)
        onHiddenRequested: (accountId, hidden) => daemon.setHidden(accountId, hidden)
        onStartServiceRequested: daemon.startService()
        onSettingsRequested: Plasmoid.internalAction("configure").trigger()
    }

    DaemonClient {
        id: daemon

        active: root.expanded
        onOpenRequested: root.expanded = true
    }

    Timer {
        interval: root.expanded ? root.activeTickMs : root.idleTickMs
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
