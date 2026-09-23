import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Spend.js" as Spend
import "logic/Tokens.js" as Tokens

ColumnLayout {
    id: spendCard

    required property var spend
    required property string period
    readonly property var current: spend[period]
    readonly property string body: Spend.bodyKind(current)

    signal periodSelected(string key)

    Layout.fillWidth: true
    spacing: Kirigami.Units.smallSpacing

    RowLayout {
        Layout.leftMargin: Metrics.headerInset(Kirigami.Units)
        spacing: Kirigami.Units.mediumSpacing

        TextLabel {
            role: "title"
            text: "Total Spend"
        }

        Kirigami.Icon {
            implicitWidth: Metrics.tinyIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: "documentinfo"
            isMask: true
            color: Tokens.secondaryText(Kirigami.Theme)

            HoverTip {
                text: Spend.infoText(spendCard.current)
            }
        }
    }

    Card {
        verticalPadding: Metrics.cardPadding(Kirigami.Units)
        spacing: Metrics.cardPadding(Kirigami.Units)

        PeriodSwitcher {
            Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
            Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
            current: spendCard.period
            onSelected: key => spendCard.periodSelected(key)
        }

        RowLayout {
            visible: spendCard.body === "ring"
            Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
            Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
            spacing: Kirigami.Units.gridUnit

            Donut {
                period: spendCard.current
            }

            SpendLegend {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                period: spendCard.current
            }
        }

        SpendStats {
            visible: spendCard.body === "stats"
            Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
            Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
            period: spendCard.current
        }

        TextLabel {
            visible: spendCard.body === "empty"
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: Kirigami.Units.gridUnit - Metrics.cardPadding(Kirigami.Units)
            Layout.bottomMargin: Kirigami.Units.gridUnit - Metrics.cardPadding(Kirigami.Units)
            emphasis: "secondary"
            text: "No usage in this period"
        }
    }
}
