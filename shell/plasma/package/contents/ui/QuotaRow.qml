import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Combined.js" as Combined
import "logic/CompactRow.js" as CompactRow
import "logic/Density.js" as Density
import "logic/Format.js" as Format
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
    readonly property bool compact: Density.isCompact(display)
    readonly property real step: Density.fontStep(compact)
    readonly property bool combined: window.segments !== undefined
    readonly property var note: Quota.paceNote(lang, window, now, display.showForecast)
    readonly property var percent: combined ? Combined.combinedPercent(window, display.valueMode) : Quota.shownPercent(window, display.valueMode)
    readonly property var forecast: compact ? null : Quota.forecast(lang, window, now, display)
    readonly property string readingText: percent === null ? "—" : reading(tweenedPercent)
    readonly property string resetText: compact ? CompactRow.trailing(lang, window, now, display.resetFormat, live, display.timeFormat) : Quota.trailingText(lang, window, now, display.resetFormat, live, display.timeFormat)
    readonly property string tipText: combined ? Combined.breakdown(lang, window, members, now, display) : compact ? CompactRow.meterTip(lang, window, now, display) : ""
    property real tweenedPercent: percent ?? 0

    signal valueModeToggled
    signal resetFormatToggled

    function reading(value) {
        if (Format.isPooled(window))
            return Format.capacityReading(lang, value, window.capacityPercent, display.valueMode);
        return Format.readingFor(lang, value, display.valueMode);
    }

    Layout.fillWidth: true
    Layout.leftMargin: Metrics.rowInset(Kirigami.Units) - Kirigami.Units.smallSpacing
    Layout.rightMargin: Metrics.rowInset(Kirigami.Units) - Kirigami.Units.smallSpacing
    Layout.topMargin: Density.barRowTop(Kirigami.Units, compact)
    Layout.bottomMargin: Density.barRowBottom(Kirigami.Units, compact)
    spacing: compact ? Math.round(Kirigami.Units.smallSpacing * 0.75) : Kirigami.Units.smallSpacing

    RowLayout {
        Layout.leftMargin: Kirigami.Units.smallSpacing
        Layout.rightMargin: row.compact ? 0 : Kirigami.Units.smallSpacing
        spacing: row.compact ? Kirigami.Units.mediumSpacing : Kirigami.Units.smallSpacing

        TextLabel {
            Layout.fillWidth: true
            role: "label"
            step: row.step
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
            visible: row.note !== null && !row.compact
            emphasis: "secondary"
            text: row.note?.text ?? ""
        }

        ToggleText {
            id: compactReadingToggle

            objectName: "compactReading"
            visible: row.compact
            step: row.step
            text: row.readingText
            hint: CompactRow.valueHint(row.lang, row.display.valueMode)
            onClicked: row.valueModeToggled()
        }

        ToggleText {
            id: compactResetToggle

            visible: row.compact
            emphasis: "secondary"
            step: row.step
            text: row.resetText
            hint: CompactRow.resetHint(row.lang, row.display.resetFormat)
            onClicked: row.resetFormatToggled()
        }
    }

    Meter {
        visible: !row.combined
        Layout.leftMargin: Kirigami.Units.smallSpacing
        Layout.rightMargin: Kirigami.Units.smallSpacing
        barHeight: Density.meterHeight(Kirigami.Units, row.compact)
        fraction: Quota.fillFraction(row.window, row.display.valueMode)
        progress: row.appear
        tone: Quota.meterTone(row.window)
        tick: Quota.tickPosition(row.window, row.display)
    }

    SegmentedMeter {
        visible: row.combined
        Layout.leftMargin: Kirigami.Units.smallSpacing
        Layout.rightMargin: Kirigami.Units.smallSpacing
        barHeight: Density.meterHeight(Kirigami.Units, row.compact)
        segments: row.combined ? Combined.segments(row.window, row.members, row.display) : []
        progress: row.appear
    }

    RowLayout {
        visible: !row.compact
        spacing: Kirigami.Units.smallSpacing

        ToggleText {
            id: readingToggle

            objectName: "reading"
            text: row.readingText
            hint: CompactRow.valueHint(row.lang, row.display.valueMode)
            onClicked: row.valueModeToggled()
        }

        Item {
            Layout.fillWidth: true
        }

        ToggleText {
            id: resetToggle

            objectName: "resetText"
            emphasis: "secondary"
            text: row.resetText
            hint: CompactRow.resetHint(row.lang, row.display.resetFormat)
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
        text: compactReadingToggle.hovered || compactResetToggle.hovered || readingToggle.hovered || resetToggle.hovered ? "" : row.tipText
    }

    Behavior on tweenedPercent {
        enabled: row.appear >= 1

        NumberAnimation {
            duration: Kirigami.Units.longDuration
            easing.type: Easing.OutCubic
        }
    }
}
