import QtQuick
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/Tokens.js" as Tokens

T.AbstractButton {
    id: button

    property bool busy: false
    property bool animated: true
    property bool pending: false
    property bool failed: false
    readonly property bool spinning: pending || busy
    readonly property bool active: pointer.shown || button.down || button.visualFocus
    readonly property int holdMs: 800
    readonly property int failureMs: 1500

    signal refreshNowRequested

    function fail() {
        hold.stop();
        pending = false;
        failed = true;
        failure.restart();
    }

    function trigger() {
        if (pending)
            return;
        failure.stop();
        failed = false;
        pending = true;
        hold.restart();
        refreshNowRequested();
    }

    function glyphColor() {
        if (failed)
            return Kirigami.Theme.negativeTextColor;
        return active ? Kirigami.Theme.textColor : Tokens.secondaryText(Kirigami.Theme);
    }

    implicitWidth: Metrics.compactControl(Kirigami.Units)
    implicitHeight: implicitWidth
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    display: T.AbstractButton.IconOnly
    Accessible.role: Accessible.Button
    Accessible.name: button.text
    Accessible.onPressAction: button.trigger()
    onClicked: trigger()

    contentItem: Item {
        SpinningIcon {
            objectName: "refreshGlyph"
            anchors.centerIn: parent
            implicitWidth: Metrics.compactIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: "view-refresh"
            isMask: true
            color: button.glyphColor()
            spinning: button.spinning
            animated: button.animated

            Behavior on color {
                enabled: button.animated

                ColorAnimation {
                    duration: Kirigami.Units.longDuration
                    easing.type: Easing.OutCubic
                }
            }
        }
    }

    background: Rectangle {
        radius: width / 2
        color: button.down ? Tokens.pressed(Kirigami.Theme) : Tokens.hover(Kirigami.Theme)
        border.width: button.visualFocus ? Metrics.hairline(Kirigami.Units) : 0
        border.color: Kirigami.Theme.highlightColor
        opacity: button.active ? 1 : 0

        Behavior on opacity {
            enabled: button.animated

            NumberAnimation {
                duration: Motion.hoverDuration(Kirigami.Units)
                easing.type: Easing.OutCubic
            }
        }
    }

    HoverTip {
        id: pointer

        text: button.text
    }

    Timer {
        id: hold

        interval: button.holdMs
        onTriggered: button.pending = false
    }

    Timer {
        id: failure

        interval: button.failureMs
        onTriggered: button.failed = false
    }
}
