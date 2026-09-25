import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

Rectangle {
    id: toast

    property bool animated: true
    property string text: ""
    property string detail: ""
    property bool failed: false
    property string actionText: ""
    property string actionUrl: ""
    property real shown: 0
    readonly property int holdMs: actionUrl === "" ? Kirigami.Units.humanMoment : Kirigami.Units.humanMoment * 2

    signal actionTriggered(string url)

    function show(message, extra, failure, action, url) {
        text = message;
        detail = extra;
        failed = failure;
        actionText = action;
        actionUrl = url;
        hold.restart();
        fade(1, Kirigami.Units.longDuration);
    }

    function dismiss() {
        hold.stop();
        fade(0, 0);
    }

    function fade(target, duration) {
        fader.stop();
        if (!animated || duration === 0) {
            shown = target;
            return;
        }
        fader.to = target;
        fader.duration = duration;
        fader.start();
    }

    objectName: "toast"
    visible: shown > 0
    opacity: shown
    implicitWidth: content.implicitWidth + Math.round(Kirigami.Units.smallSpacing * 5.25)
    implicitHeight: content.implicitHeight + Math.round(Kirigami.Units.smallSpacing * 3)
    radius: height / 2
    color: Kirigami.Theme.backgroundColor
    border.width: Metrics.hairline(Kirigami.Units)
    border.color: Tokens.separator(Kirigami.Theme)

    transform: Translate {
        y: (1 - toast.shown) * Kirigami.Units.largeSpacing
    }

    RowLayout {
        id: content

        anchors.centerIn: parent
        anchors.horizontalCenterOffset: -Math.round(Kirigami.Units.smallSpacing * 0.375)
        spacing: Kirigami.Units.mediumSpacing

        Kirigami.Icon {
            implicitWidth: Metrics.compactIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: toast.failed ? "dialog-error" : "checkmark"
            isMask: true
            color: toast.failed ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.positiveTextColor
        }

        TextLabel {
            role: "caption"
            weight: Font.DemiBold
            text: toast.text
        }

        TextLabel {
            visible: toast.detail !== ""
            role: "caption"
            emphasis: "secondary"
            text: `· ${toast.detail}`
        }

        LinkText {
            objectName: "toastAction"
            visible: toast.actionText !== ""
            text: toast.actionText
            onClicked: toast.actionTriggered(toast.actionUrl)
        }
    }

    Timer {
        id: hold

        interval: toast.holdMs
        onTriggered: toast.fade(0, Kirigami.Units.longDuration * 0.75)
    }

    NumberAnimation {
        id: fader

        target: toast
        property: "shown"
        easing.type: Easing.OutCubic
    }
}
