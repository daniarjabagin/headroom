import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

Item {
    id: status

    required property var notice
    required property string lang
    required property bool compact
    property bool animated: true
    readonly property string linkText: `${I18n.tr(lang, "Status page")} ↗`
    property real shown: 0

    signal linkActivated(string url)

    objectName: "statusNotice"
    readonly property real sideInset: compact ? Metrics.rowInset(Kirigami.Units) : Kirigami.Units.largeSpacing
    readonly property real topInset: Kirigami.Units.mediumSpacing
    readonly property real bottomInset: compact ? Math.round(Kirigami.Units.smallSpacing / 2) : Kirigami.Units.mediumSpacing

    implicitHeight: (compact ? line.implicitHeight : plate.implicitHeight) + topInset + bottomInset
    opacity: shown

    transform: Translate {
        y: (1 - status.shown) * Kirigami.Units.smallSpacing
    }

    Component.onCompleted: {
        if (animated)
            appearAnimation.start();
        else
            shown = 1;
    }

    Rectangle {
        id: plate

        visible: !status.compact
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: status.sideInset
        anchors.rightMargin: status.sideInset
        anchors.topMargin: status.topInset
        implicitHeight: content.implicitHeight + content.anchors.margins * 2
        radius: Metrics.noticeRadius(Kirigami.Units)
        color: Tokens.noticeFill(Kirigami.Theme, status.notice.kind)
        border.width: Metrics.hairline(Kirigami.Units)
        border.color: Tokens.noticeBorder(Kirigami.Theme, status.notice.kind)

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
                color: Tokens.noticeTile(Kirigami.Theme, status.notice.kind)

                Kirigami.Icon {
                    anchors.centerIn: parent
                    implicitWidth: Math.round(Kirigami.Units.iconSizes.small * 0.875)
                    implicitHeight: implicitWidth
                    source: status.notice.kind === "error" ? "dialog-error" : "dialog-warning"
                    isMask: true
                    color: Tokens.noticeColor(Kirigami.Theme, status.notice.kind)
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Math.round(Kirigami.Units.smallSpacing / 4)

                TextLabel {
                    Layout.fillWidth: true
                    role: "caption"
                    weight: Font.DemiBold
                    text: status.notice.title
                    wrapMode: Text.Wrap
                }

                TextLabel {
                    visible: text !== ""
                    Layout.fillWidth: true
                    role: "caption"
                    emphasis: "secondary"
                    text: status.notice.detail
                    wrapMode: Text.Wrap
                }

                RowLayout {
                    Layout.topMargin: Math.round(Kirigami.Units.smallSpacing * 0.75)
                    spacing: Kirigami.Units.mediumSpacing

                    TextLabel {
                        Layout.fillWidth: true
                        role: "caption"
                        emphasis: "secondary"
                        text: status.notice.started
                        elide: Text.ElideRight
                    }

                    LinkText {
                        objectName: "statusLink"
                        text: status.linkText
                        onClicked: status.linkActivated(status.notice.url)
                    }
                }
            }
        }
    }

    RowLayout {
        id: line

        visible: status.compact
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: status.sideInset
        anchors.rightMargin: status.sideInset
        anchors.topMargin: status.topInset
        spacing: Kirigami.Units.mediumSpacing

        Rectangle {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Math.round(Kirigami.Units.smallSpacing * 1.75)
            implicitHeight: implicitWidth
            radius: width / 2
            color: Tokens.toneColor(Kirigami.Theme, status.notice.tone)
        }

        TextLabel {
            id: headline

            Layout.fillWidth: true
            Layout.preferredWidth: headlineMetrics.advanceWidth
            Layout.maximumWidth: Math.ceil(headlineMetrics.advanceWidth) + Metrics.hairline(Kirigami.Units)
            role: "caption"
            weight: Font.DemiBold
            text: status.notice.headline
            elide: Text.ElideRight

            TextMetrics {
                id: headlineMetrics

                font: headline.font
                text: headline.text
            }
        }

        TextLabel {
            visible: status.notice.age !== ""
            role: "caption"
            emphasis: "secondary"
            text: `· ${status.notice.age}`
        }

        Item {
            Layout.fillWidth: true
            Layout.preferredWidth: 0
        }

        LinkText {
            text: status.linkText
            onClicked: status.linkActivated(status.notice.url)
        }
    }

    HoverTip {
        text: status.compact ? [status.notice.title, status.notice.detail, status.notice.started].filter(part => part !== "").join("\n") : ""
    }

    NumberAnimation {
        id: appearAnimation

        target: status
        property: "shown"
        from: 0
        to: 1
        duration: Kirigami.Units.longDuration
        easing.type: Easing.OutCubic
    }
}
