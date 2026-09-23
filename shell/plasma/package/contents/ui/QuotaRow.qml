import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Format.js" as Format
import "logic/Metrics.js" as Metrics
import "logic/Quota.js" as Quota
import "logic/Tokens.js" as Tokens

ColumnLayout {
    id: row

    required property var window
    required property var now
    required property bool alwaysShowPacing
    readonly property var note: Quota.paceNote(window, now, alwaysShowPacing)

    Layout.fillWidth: true
    Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
    Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
    Layout.topMargin: Metrics.barRowPadding(Kirigami.Units)
    Layout.bottomMargin: Metrics.barRowPadding(Kirigami.Units)
    spacing: Kirigami.Units.smallSpacing

    RowLayout {
        spacing: Kirigami.Units.smallSpacing

        TextLabel {
            Layout.fillWidth: true
            role: "label"
            text: row.window.label
            elide: Text.ElideRight
        }

        Kirigami.Icon {
            visible: row.note?.flame ?? false
            implicitWidth: Metrics.tinyIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: Qt.resolvedUrl("../icons/flame-symbolic.svg")
            isMask: true
            color: Tokens.toneColor(Kirigami.Theme, "critical")
        }

        TextLabel {
            visible: row.note !== null
            emphasis: "secondary"
            text: row.note?.text ?? ""
        }
    }

    Meter {
        fraction: Quota.fillFraction(row.window)
        tone: Quota.meterTone(row.window)
        tick: Quota.tickPosition(row.window, row.alwaysShowPacing)
    }

    RowLayout {
        spacing: Kirigami.Units.largeSpacing

        TextLabel {
            Layout.fillWidth: true
            text: Format.percentLeft(row.window.remainingPercent)
        }

        TextLabel {
            emphasis: "secondary"
            text: Quota.trailingText(row.window, row.now)
        }
    }
}
