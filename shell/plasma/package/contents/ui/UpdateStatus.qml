import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

RowLayout {
    id: status

    required property var run
    required property string line
    required property bool animated
    readonly property color tint: run.phase === "failed" ? Kirigami.Theme.negativeTextColor : Tokens.secondaryText(Kirigami.Theme)

    spacing: Kirigami.Units.smallSpacing

    SpinningIcon {
        objectName: "updateSpinner"
        visible: status.run.phase === "running"
        Layout.alignment: Qt.AlignTop
        Layout.topMargin: Math.round(Kirigami.Units.smallSpacing / 2)
        implicitWidth: Metrics.tinyIcon(Kirigami.Units)
        implicitHeight: implicitWidth
        source: "view-refresh"
        isMask: true
        color: status.tint
        spinning: visible
        animated: status.animated
    }

    Kirigami.Icon {
        visible: status.run.phase === "done" || status.run.phase === "failed"
        Layout.alignment: Qt.AlignTop
        Layout.topMargin: Math.round(Kirigami.Units.smallSpacing / 2)
        implicitWidth: Metrics.tinyIcon(Kirigami.Units)
        implicitHeight: implicitWidth
        source: status.run.phase === "done" ? "checkmark" : "dialog-error"
        isMask: true
        color: status.run.phase === "done" ? Kirigami.Theme.positiveTextColor : status.tint
    }

    TextLabel {
        objectName: "updateLine"
        Layout.fillWidth: true
        role: "caption"
        color: status.tint
        wrapMode: Text.Wrap
        text: status.line
    }
}
