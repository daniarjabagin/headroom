pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import headroom.preview
import "../../package/contents/ui" as Ui
import "../../package/contents/ui/logic/Settings.js" as Settings

ColumnLayout {
    id: samples

    readonly property var claude: item("claude", "Claude", "weekly", "Weekly", 72, "good", 41.5)
    readonly property var codex: item("codex", "Codex", "weekly", "Weekly", 40, "warning", 52)
    readonly property var critical: item("claude", "Claude", "session", "Session", 8, "critical", 60)
    readonly property var pooled: Object.assign(item("codex", "Codex", "session", "Session", 62, "good", null), {
        combined: true,
        accountCount: 2
    })
    readonly property var horizontal: [strip("headline", "ring", "percent", [claude]), strip("headline", "ring", "percent", [codex]), strip("headline", "ring", "percent", [critical]), strip("headline", "bar", "percent", [claude]), strip("headline", "bar", "percent", [codex]), strip("headline", "bar", "percent", [critical]), strip("headline", "none", "percent", [claude]), strip("headline", "none", "percent", [codex]), strip("headline", "none", "percent", [critical]), strip("headline", "ring", "none", [codex]), strip("headline", "ring", "window", [pooled]), strip("headline", "none", "window", [critical]), strip("several", "none", "percent", [claude, codex]), strip("several", "ring", "percent", [claude, codex]), strip("several", "bar", "percent", [claude, codex, critical]), strip("icon", "ring", "percent", [], "warning"), strip("icon", "ring", "percent", [], "critical"), strip("headline", "ring", "percent", [], null)]
    readonly property var verticals: [vertical(strip("several", "none", "percent", [claude, codex]), 48), vertical(strip("several", "bar", "percent", [claude, codex]), 48), vertical(strip("headline", "ring", "percent", [codex]), 48), vertical(strip("several", "none", "percent", [claude, codex, critical]), 32)]
    readonly property var tipRows: ({
            pinned: [row("claude", "Claude · Weekly", "72% left", "good", "Resets in 2d 13h"), row("codex", "Codex · Weekly", "40% left", "warning", "Runs out in 2d 1h · Resets in 4d 5h")],
            rest: [row("copilot", "Copilot · Credits", "56% left", "good", ""), row("grok", "Grok · Weekly", "64% left", "good", "")]
        })

    function item(provider, providerName, windowId, windowLabel, remaining, tone, evenPace) {
        return {
            accountId: `${provider}:${windowId}`,
            provider,
            providerName,
            windowId,
            windowLabel,
            usedPercent: 100 - remaining,
            remainingPercent: remaining,
            valuePercent: remaining,
            evenPacePercent: evenPace,
            tone,
            logo: provider,
            combined: false,
            accountCount: 1
        };
    }

    function strip(mode, indicator, label, items, tone) {
        return {
            items,
            tone: tone ?? null,
            display: Settings.parseDisplay({
                panel_mode: mode,
                panel_indicator: indicator,
                panel_label: label
            })
        };
    }

    function vertical(sample, thickness) {
        return Object.assign({
            thickness
        }, sample);
    }

    function row(provider, title, reading, tone, detail) {
        return {
            key: title,
            provider,
            title,
            reading,
            tone,
            detail
        };
    }

    spacing: Kirigami.Units.largeSpacing

    RowLayout {
        spacing: Kirigami.Units.largeSpacing

        GridLayout {
            Layout.alignment: Qt.AlignTop
            columns: 3
            rowSpacing: Kirigami.Units.smallSpacing
            columnSpacing: Kirigami.Units.smallSpacing

            Repeater {
                model: samples.horizontal

                Rectangle {
                    id: panel

                    required property var modelData

                    Layout.fillWidth: true
                    implicitWidth: compact.Layout.minimumWidth + Kirigami.Units.largeSpacing * 4
                    implicitHeight: 44
                    radius: PreviewConfig.dialogRadius
                    color: Kirigami.Theme.backgroundColor

                    Ui.CompactRepresentation {
                        id: compact

                        anchors.centerIn: parent
                        width: Layout.minimumWidth
                        height: parent.height
                        items: panel.modelData.items
                        panelTone: panel.modelData.tone
                        display: panel.modelData.display
                    }
                }
            }
        }

        Repeater {
            model: samples.verticals

            Rectangle {
                id: column

                required property var modelData

                Layout.alignment: Qt.AlignTop
                implicitWidth: modelData.thickness
                implicitHeight: verticalCompact.Layout.minimumHeight + Kirigami.Units.largeSpacing * 4
                radius: PreviewConfig.dialogRadius
                color: Kirigami.Theme.backgroundColor

                Ui.CompactRepresentation {
                    id: verticalCompact

                    anchors.centerIn: parent
                    width: parent.width
                    height: Layout.minimumHeight
                    vertical: true
                    items: column.modelData.items
                    display: column.modelData.display
                }
            }
        }
    }

    Rectangle {
        implicitWidth: tooltip.implicitWidth
        implicitHeight: tooltip.implicitHeight
        radius: PreviewConfig.dialogRadius
        color: Kirigami.Theme.backgroundColor

        Ui.PanelTooltip {
            id: tooltip

            anchors.fill: parent
            rows: samples.tipRows
        }
    }
}
