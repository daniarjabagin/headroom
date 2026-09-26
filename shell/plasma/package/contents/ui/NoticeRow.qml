import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

Rectangle {
    id: notice

    required property var entry
    readonly property var actions: entry.actions
    readonly property bool inlineAction: actions.length === 1
    property bool animated: true
    property string copiedValue: ""

    signal actionTriggered(string kind, string value)

    function trigger(kind, value) {
        if (kind !== "copy") {
            actionTriggered(kind, value);
            return;
        }
        clipboard.text = value;
        clipboard.selectAll();
        clipboard.copy();
        copiedValue = value;
    }

    function iconFor(kind) {
        if (kind === "error")
            return "dialog-error";
        if (kind === "signin")
            return "im-user";
        return "dialog-warning";
    }

    Layout.fillWidth: true
    Layout.leftMargin: Kirigami.Units.largeSpacing
    Layout.rightMargin: Kirigami.Units.largeSpacing
    Layout.topMargin: Kirigami.Units.mediumSpacing
    Layout.bottomMargin: Kirigami.Units.mediumSpacing
    implicitHeight: content.implicitHeight + content.anchors.margins * 2
    radius: Metrics.noticeRadius(Kirigami.Units)
    color: Tokens.noticeFill(Kirigami.Theme, entry.kind)
    border.width: Metrics.hairline(Kirigami.Units)
    border.color: Tokens.noticeBorder(Kirigami.Theme, entry.kind)

    RowLayout {
        id: content

        anchors.fill: parent
        anchors.margins: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.largeSpacing

        Rectangle {
            Layout.alignment: Qt.AlignTop
            implicitWidth: Metrics.tileSize(Kirigami.Units)
            implicitHeight: implicitWidth
            radius: Math.round(Kirigami.Units.smallSpacing * 1.75)
            color: Tokens.noticeTile(Kirigami.Theme, notice.entry.kind)

            Kirigami.Icon {
                anchors.centerIn: parent
                implicitWidth: Math.round(Kirigami.Units.iconSizes.small * 0.875)
                implicitHeight: implicitWidth
                source: notice.iconFor(notice.entry.kind)
                isMask: true
                color: Tokens.noticeColor(Kirigami.Theme, notice.entry.kind)
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            spacing: Math.round(Kirigami.Units.smallSpacing / 4)

            TextLabel {
                Layout.fillWidth: true
                role: "caption"
                weight: Font.DemiBold
                text: notice.entry.title
                wrapMode: Text.Wrap
            }

            TextLabel {
                visible: text !== ""
                Layout.fillWidth: true
                role: "caption"
                emphasis: "secondary"
                text: notice.entry.detail
                wrapMode: Text.Wrap
            }

            TextLabel {
                visible: text !== ""
                Layout.fillWidth: true
                role: "caption"
                emphasis: "tertiary"
                text: notice.entry.note
                wrapMode: Text.Wrap
            }

            NoticeActions {
                visible: !notice.inlineAction && notice.actions.length > 0
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.mediumSpacing
                actions: notice.actions
                copiedValue: notice.copiedValue
                animated: notice.animated
                onTriggered: (kind, value) => notice.trigger(kind, value)
            }
        }

        NoticeActions {
            visible: notice.inlineAction
            Layout.alignment: Qt.AlignVCenter
            actions: notice.actions
            copiedValue: notice.copiedValue
            animated: notice.animated
            onTriggered: (kind, value) => notice.trigger(kind, value)
        }
    }

    TextEdit {
        id: clipboard

        visible: false
    }
}
