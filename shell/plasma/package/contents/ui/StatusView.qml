import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

Card {
    id: status

    required property var view
    readonly property string kind: view.kind === "ready" ? "empty" : view.kind

    signal startServiceRequested
    signal refreshRequested

    verticalPadding: Kirigami.Units.gridUnit + Kirigami.Units.mediumSpacing
    spacing: Metrics.barRowPadding(Kirigami.Units)

    Kirigami.Icon {
        visible: status.kind === "unavailable"
        Layout.alignment: Qt.AlignHCenter
        implicitWidth: Kirigami.Units.iconSizes.medium
        implicitHeight: implicitWidth
        source: Qt.resolvedUrl("../icons/headroom-symbolic.svg")
        isMask: true
        color: Tokens.secondaryText(Kirigami.Theme)
    }

    CenteredText {
        visible: text !== ""
        role: "label"
        text: status.titleText()
    }

    CenteredText {
        visible: text !== ""
        emphasis: "secondary"
        text: status.detailText()
    }

    PrimaryButton {
        visible: status.kind === "unavailable" || status.kind === "error"
        Layout.alignment: Qt.AlignHCenter
        Layout.topMargin: Kirigami.Units.smallSpacing
        enabled: !(status.view.starting ?? false)
        text: status.kind === "error" ? "Try again" : (status.view.starting ? "Starting…" : "Start service")
        onClicked: status.kind === "error" ? status.refreshRequested() : status.startServiceRequested()
    }

    SmallButton {
        visible: status.kind === "empty"
        Layout.alignment: Qt.AlignHCenter
        text: "Check again"
        onClicked: status.refreshRequested()
    }

    CenteredText {
        visible: text !== ""
        role: "caption"
        color: Kirigami.Theme.negativeTextColor
        text: status.view.startError ?? ""
    }

    function titleText() {
        if (kind === "unavailable")
            return "Headroom service isn't running";
        if (kind === "error")
            return "Couldn't read Headroom's state";
        return "";
    }

    function detailText() {
        if (kind === "unavailable")
            return "Start it to see your usage limits here.";
        if (kind === "error")
            return view.error ?? "";
        if (kind === "empty")
            return "No AI coding tools found.";
        return "Loading…";
    }

    component CenteredText: TextLabel {
        Layout.fillWidth: true
        Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
        Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.Wrap
    }
}
