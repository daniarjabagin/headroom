import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Spend.js" as Spend
import "logic/Tokens.js" as Tokens

ColumnLayout {
    id: spendCard

    required property var spend
    required property string period
    required property string lang
    required property real appear
    readonly property var current: spend[period]
    readonly property string body: Spend.bodyKind(current)

    signal periodSelected(string key)

    Layout.fillWidth: true
    spacing: Kirigami.Units.smallSpacing
    opacity: appear

    transform: [
        Translate {
            y: (1 - spendCard.appear) * Kirigami.Units.gridUnit * 0.75
        }
    ]

    RowLayout {
        Layout.leftMargin: Metrics.headerInset(Kirigami.Units)
        spacing: Kirigami.Units.mediumSpacing

        TextLabel {
            role: "title"
            text: I18n.tr(spendCard.lang, "Total Spend")
        }

        Kirigami.Icon {
            implicitWidth: Metrics.tinyIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: "documentinfo"
            isMask: true
            color: Tokens.secondaryText(Kirigami.Theme)

            HoverTip {
                text: Spend.infoText(spendCard.lang, spendCard.current)
            }
        }
    }

    Card {
        verticalPadding: Metrics.cardPadding(Kirigami.Units)
        spacing: Metrics.cardPadding(Kirigami.Units)

        SegmentedControl {
            Layout.fillWidth: true
            Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
            Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
            options: Spend.periodOptions(spendCard.lang)
            current: spendCard.period
            onSelected: value => spendCard.periodSelected(value)
        }

        RowLayout {
            visible: spendCard.body === "ring"
            Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
            Layout.rightMargin: Metrics.rowInset(Kirigami.Units) - Kirigami.Units.smallSpacing
            spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing

            Donut {
                period: spendCard.current
                progress: spendCard.appear
            }

            SpendLegend {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                period: spendCard.current
                periodKey: spendCard.period
                lang: spendCard.lang
            }
        }

        TextLabel {
            visible: spendCard.body === "empty"
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: Kirigami.Units.gridUnit - Metrics.cardPadding(Kirigami.Units)
            Layout.bottomMargin: Kirigami.Units.gridUnit - Metrics.cardPadding(Kirigami.Units)
            emphasis: "secondary"
            text: I18n.tr(spendCard.lang, "No usage in this period")
        }
    }
}
