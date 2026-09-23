import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Format.js" as Format
import "logic/Providers.js" as Providers

RowLayout {
    id: stats

    required property var period
    readonly property string providerName: period.providers.length > 0 ? Providers.providerInfo(period.providers[0].provider).name : ""

    spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing

    StatColumn {
        value: Format.exactUsd(stats.period.costMicros)
        caption: `dollars · ${stats.providerName}`
    }

    StatColumn {
        value: Format.exactTokens(stats.period.totalTokens)
        caption: "tokens"
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
