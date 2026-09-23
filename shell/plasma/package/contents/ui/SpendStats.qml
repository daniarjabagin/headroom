import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Format.js" as Format
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Providers.js" as Providers
import "logic/Spend.js" as Spend
import "logic/Tokens.js" as Tokens

Item {
    id: stats

    required property var period
    required property string periodKey
    required property string lang
    readonly property var provider: period.providers.length > 0 ? period.providers[0] : null
    readonly property string providerName: provider ? Providers.providerInfo(provider.provider).name : ""

    Layout.fillWidth: true
    implicitHeight: row.implicitHeight + Kirigami.Units.smallSpacing * 2

    Rectangle {
        anchors.fill: parent
        radius: Metrics.chipRadius(Kirigami.Units)
        color: Tokens.chip(Kirigami.Theme)
        opacity: tip.hovered ? 1 : 0

        Behavior on opacity {
            NumberAnimation {
                duration: Kirigami.Units.shortDuration
            }
        }
    }

    RowLayout {
        id: row

        anchors.fill: parent
        anchors.margins: Kirigami.Units.smallSpacing
        spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing

        StatColumn {
            value: Format.exactUsd(stats.period.costMicros)
            caption: `${I18n.tr(stats.lang, "dollars")} · ${stats.providerName}`
        }

        StatColumn {
            value: Format.exactTokens(stats.period.totalTokens)
            caption: I18n.trn(stats.lang, "token", "tokens", stats.period.totalTokens)
        }
    }

    ModelTip {
        id: tip

        lang: stats.lang
        title: stats.provider ? Spend.breakdownTitle(stats.lang, stats.periodKey, stats.provider.provider) : ""
        totals: stats.provider
        fallback: stats.provider ? Format.spendTooltip(stats.lang, stats.provider) : ""
    }

    component StatColumn: ColumnLayout {
        id: column

        required property string value
        required property string caption

        Layout.fillWidth: true
        Layout.preferredWidth: 1
        spacing: Kirigami.Units.smallSpacing / 2

        TextLabel {
            role: "title"
            text: column.value
        }

        TextLabel {
            role: "caption"
            emphasis: "secondary"
            text: column.caption
        }
    }
}
