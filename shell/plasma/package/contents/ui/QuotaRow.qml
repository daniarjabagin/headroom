import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Combined.js" as Combined
import "logic/Format.js" as Format
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Quota.js" as Quota
import "logic/Tokens.js" as Tokens

ColumnLayout {
    id: row

    required property var window
    required property var now
    required property bool live
    required property var display
    required property string lang
    required property real appear
    property var members: []
    readonly property bool combined: window.segments !== undefined
    readonly property var note: Quota.paceNote(lang, window, now, display.showForecast)
    readonly property var percent: combined ? Combined.combinedPercent(window, display.valueMode) : Quota.shownPercent(window, display.valueMode)
    readonly property var forecast: Quota.forecast(lang, window, now, display)
    property real tweenedPercent: percent ?? 0

    signal valueModeToggled
    signal resetFormatToggled

    function reading(value) {
        if (combined)
            return Format.capacityReading(lang, value, window.capacityPercent, display.valueMode);
        return Format.readingFor(lang, value, display.valueMode);
    }

    Layout.fillWidth: true
    Layout.leftMargin: Metrics.rowInset(Kirigami.Units) - Kirigami.Units.smallSpacing
    Layout.rightMargin: Metrics.rowInset(Kirigami.Units) - Kirigami.Units.smallSpacing
    Layout.topMargin: Metrics.barRowPadding(Kirigami.Units)
    Layout.bottomMargin: Metrics.barRowPadding(Kirigami.Units)
    spacing: Kirigami.Units.smallSpacing

    RowLayout {
        Layout.leftMargin: Kirigami.Units.smallSpacing
        Layout.rightMargin: Kirigami.Units.smallSpacing
        spacing: Kirigami.Units.smallSpacing

        TextLabel {
            Layout.fillWidth: true
            role: "label"
            text: Format.windowLabel(row.lang, row.window)
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
        visible: !row.combined
        Layout.leftMargin: Kirigami.Units.smallSpacing
        Layout.rightMargin: Kirigami.Units.smallSpacing
        fraction: Quota.fillFraction(row.window, row.display.valueMode)
        progress: row.appear
        tone: Quota.meterTone(row.window)
        tick: Quota.tickPosition(row.window, row.display)
    }

    SegmentedMeter {
        visible: row.combined
        Layout.leftMargin: Kirigami.Units.smallSpacing
        Layout.rightMargin: Kirigami.Units.smallSpacing
        segments: row.combined ? Combined.segments(row.window, row.members, row.display) : []
        progress: row.appear
    }

    RowLayout {
        spacing: Kirigami.Units.smallSpacing

        ToggleText {
            text: row.percent === null ? "—" : row.reading(row.tweenedPercent)
            hint: I18n.tr(row.lang, "Click to switch between left and used")
            onClicked: row.valueModeToggled()
        }

        Item {
            Layout.fillWidth: true
        }

        ToggleText {
            emphasis: "secondary"
            text: Quota.trailingText(row.lang, row.window, row.now, row.display.resetFormat, row.live)
            hint: I18n.tr(row.lang, "Click to switch between countdown and exact time")
            onClicked: row.resetFormatToggled()
        }
    }

    TextLabel {
        visible: row.forecast !== null
        Layout.fillWidth: true
        Layout.leftMargin: Kirigami.Units.smallSpacing
        Layout.rightMargin: Kirigami.Units.smallSpacing
        role: "caption"
        emphasis: "secondary"
        wrapMode: Text.Wrap
        text: row.forecast ?? ""
    }

    HoverTip {
        text: row.combined ? Combined.breakdown(row.lang, row.window, row.members, row.now, row.display) : ""
    }

    Behavior on tweenedPercent {
        enabled: row.appear >= 1

        NumberAnimation {
            duration: Kirigami.Units.longDuration
            easing.type: Easing.OutCubic
        }
    }
}
