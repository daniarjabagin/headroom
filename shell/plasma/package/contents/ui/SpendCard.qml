import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Density.js" as Density
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Spend.js" as Spend
import "logic/SpendBreakdown.js" as SpendBreakdown
import "logic/Tokens.js" as Tokens

ColumnLayout {
    id: spendCard

    required property var spend
    required property string period
    required property string lang
    required property real appear
    property string unit: "cost"
    property string breakdown: "models"
    property bool capable: false
    property bool showBreakdown: true
    property bool compact: false
    property bool animated: true
    readonly property var current: Spend.periodTotals(spend, period)
    readonly property string body: Spend.bodyKind(current)
    readonly property real padding: Density.spendPadding(Kirigami.Units, compact)

    signal periodSelected(string key)
    signal unitSelected(string key)
    signal breakdownSelected(string key)

    Layout.fillWidth: true
    spacing: Density.headerGap(Kirigami.Units, compact)
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
            visible: !spendCard.capable
            role: "title"
            step: Density.fontStep(spendCard.compact)
            text: I18n.tr(spendCard.lang, "Total Spend")
        }

        UnitTitle {
            visible: spendCard.capable
            unit: spendCard.unit
            lang: spendCard.lang
            animated: spendCard.animated
            step: Density.fontStep(spendCard.compact)
            onPicked: key => spendCard.unitSelected(key)
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
        animated: spendCard.animated
        verticalPadding: spendCard.padding
        spacing: spendCard.padding

        SegmentedControl {
            objectName: "periodControl"
            Layout.fillWidth: true
            Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
            Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
            options: Spend.periodOptions(spendCard.lang, spendCard.spend)
            current: Spend.hasPeriod(spendCard.spend, spendCard.period) ? spendCard.period : "30d"
            animated: spendCard.animated
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
                animated: spendCard.animated
                unit: spendCard.unit
                lang: spendCard.lang
                size: Density.donutSize(Kirigami.Units, spendCard.compact)
            }

            SpendLegend {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                period: spendCard.current
                periodKey: spendCard.period
                animated: spendCard.animated
                unit: spendCard.unit
                lang: spendCard.lang
            }
        }

        SpendBreakdownList {
            visible: spendCard.showBreakdown && spendCard.body === "ring" && SpendBreakdown.hasProjects(spendCard.current)
            period: spendCard.current
            mode: spendCard.breakdown
            unit: spendCard.unit
            animated: spendCard.animated
            lang: spendCard.lang
            onModeSelected: key => spendCard.breakdownSelected(key)
        }

        TextLabel {
            visible: spendCard.body === "empty"
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: Kirigami.Units.gridUnit - spendCard.padding
            Layout.bottomMargin: Kirigami.Units.gridUnit - spendCard.padding
            emphasis: "secondary"
            text: I18n.tr(spendCard.lang, "No usage in this period")
        }
    }
}
