pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Collapse.js" as Collapse
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/Tokens.js" as Tokens

T.AbstractButton {
    id: row

    required property var folded
    required property string lang
    property bool offline: false
    property bool animated: true
    readonly property int glyphLimit: 3
    readonly property var glyphs: Collapse.foldedProviders(folded, glyphLimit)
    readonly property var attention: Collapse.attention(folded, offline)

    objectName: "collapsedRow"
    Layout.fillWidth: true
    implicitHeight: content.implicitHeight + topPadding + bottomPadding
    topPadding: Metrics.cardPadding(Kirigami.Units)
    bottomPadding: Metrics.cardPadding(Kirigami.Units)
    leftPadding: Metrics.rowInset(Kirigami.Units)
    rightPadding: Metrics.cardPadding(Kirigami.Units)
    hoverEnabled: true
    Accessible.role: Accessible.Button
    Accessible.name: Collapse.accessibleTitle(lang, folded, attention)

    background: Rectangle {
        radius: Metrics.cardRadius(Kirigami.Units)
        color: pointer.shown || row.visualFocus ? Tokens.cardHover(Kirigami.Theme) : Tokens.card(Kirigami.Theme)

        Behavior on color {
            enabled: row.animated

            ColorAnimation {
                duration: Motion.hoverDuration(Kirigami.Units)
                easing.type: Easing.OutCubic
            }
        }
    }

    contentItem: RowLayout {
        id: content

        spacing: Kirigami.Units.largeSpacing

        Row {
            Layout.alignment: Qt.AlignVCenter
            spacing: Kirigami.Units.smallSpacing

            Repeater {
                model: row.glyphs.length

                ProviderIcon {
                    required property int index

                    implicitWidth: Kirigami.Units.iconSizes.small - 2
                    implicitHeight: implicitWidth
                    provider: row.glyphs[index] ?? ""
                }
            }
        }

        TextLabel {
            weight: Font.DemiBold
            text: Collapse.foldedCount(row.lang, row.folded)
        }

        TextLabel {
            Layout.fillWidth: true
            emphasis: "secondary"
            text: `· ${Collapse.foldedNames(row.folded)}`
            elide: Text.ElideRight
        }

        Item {
            objectName: "attentionMark"
            visible: row.attention.kind !== "none"
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Metrics.tinyIcon(Kirigami.Units)
            implicitHeight: implicitWidth

            Kirigami.Icon {
                objectName: "attentionIcon"
                anchors.fill: parent
                visible: row.attention.kind === "notice"
                source: "dialog-warning"
                isMask: true
                color: Tokens.noticeColor(Kirigami.Theme, row.attention.tone === "critical" ? "error" : "warning")
            }

            Rectangle {
                objectName: "attentionDot"
                anchors.centerIn: parent
                visible: row.attention.kind === "tone"
                width: Math.round(Kirigami.Units.smallSpacing * 1.5)
                height: width
                radius: width / 2
                color: Tokens.toneColor(Kirigami.Theme, row.attention.tone)
            }

            HoverTip {
                id: attentionTip

                text: Collapse.attentionText(row.lang, row.attention)
            }
        }

        Kirigami.Icon {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Metrics.caretIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: "arrow-right"
            isMask: true
            color: Tokens.secondaryText(Kirigami.Theme)
        }
    }

    HoverTip {
        id: pointer

        text: attentionTip.shown ? "" : I18n.tr(row.lang, "Show accounts that are not pinned")
    }
}
