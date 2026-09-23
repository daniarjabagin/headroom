import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Providers.js" as Providers
import "logic/Tokens.js" as Tokens

Item {
    id: accountRow

    required property var account
    required property var display
    required property string lang
    required property bool separated
    required property bool expanded
    required property bool canReorder
    required property bool lifted
    required property real dragOffset
    required property string indicator
    readonly property var info: Providers.providerInfo(account.provider)

    signal hiddenToggled(bool hidden)
    signal labelApplied(string label)
    signal windowToggled(string windowId, bool hidden)
    signal removeConfirmed
    signal expandToggled
    signal dragMoved(real offset)
    signal dragFinished

    function subtitle() {
        const parts = [info.name, account.plan, account.label ? account.email : null];
        if (account.owner === "headroom")
            parts.push(I18n.tr(lang, "added in Headroom"));
        return parts.filter(Boolean).join(" · ");
    }

    Layout.fillWidth: true
    implicitHeight: column.implicitHeight
    z: lifted ? 2 : 0
    opacity: lifted ? 0.9 : 1

    transform: Translate {
        y: accountRow.dragOffset
    }

    Rectangle {
        anchors.fill: parent
        visible: accountRow.lifted
        radius: Metrics.cardRadius(Kirigami.Units)
        color: Tokens.cardHover(Kirigami.Theme)
        border.width: Metrics.hairline(Kirigami.Units)
        border.color: Tokens.separator(Kirigami.Theme)
    }

    Rectangle {
        visible: accountRow.separated && !accountRow.lifted
        x: Metrics.rowInset(Kirigami.Units)
        width: parent.width - x * 2
        height: Metrics.hairline(Kirigami.Units)
        color: Tokens.separator(Kirigami.Theme)
    }

    ColumnLayout {
        id: column

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Kirigami.Units.largeSpacing
            Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
            Layout.topMargin: Kirigami.Units.largeSpacing
            Layout.bottomMargin: Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.largeSpacing

            Kirigami.Icon {
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
                source: "handle-sort"
                isMask: true
                color: Tokens.tertiaryText(Kirigami.Theme)
                opacity: accountRow.canReorder ? 1 : 0.4

                HoverHandler {
                    cursorShape: accountRow.canReorder ? Qt.OpenHandCursor : Qt.ArrowCursor
                }

                DragHandler {
                    enabled: accountRow.canReorder
                    target: null
                    xAxis.enabled: false
                    onTranslationChanged: {
                        if (active)
                            accountRow.dragMoved(translation.y);
                    }
                    onActiveChanged: {
                        if (!active)
                            accountRow.dragFinished();
                    }
                }
            }

            ProviderIcon {
                provider: accountRow.account.provider
                implicitWidth: Kirigami.Units.iconSizes.smallMedium
                implicitHeight: implicitWidth
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Math.round(Kirigami.Units.smallSpacing / 2)

                TextLabel {
                    Layout.fillWidth: true
                    role: "label"
                    weight: Font.Medium
                    text: Providers.accountName(accountRow.account)
                    elide: Text.ElideRight
                }

                TextLabel {
                    Layout.fillWidth: true
                    role: "caption"
                    emphasis: "secondary"
                    text: accountRow.subtitle()
                    elide: Text.ElideRight
                }
            }

            QQC2.Switch {
                checked: !accountRow.account.hidden
                onToggled: accountRow.hiddenToggled(!checked)

                HoverTip {
                    text: I18n.tr(accountRow.lang, "Show in the panel and popup")
                }
            }

            T.AbstractButton {
                id: expander

                implicitWidth: Kirigami.Units.iconSizes.small + Kirigami.Units.smallSpacing * 2
                implicitHeight: implicitWidth
                hoverEnabled: true
                onClicked: accountRow.expandToggled()

                contentItem: Kirigami.Icon {
                    source: "arrow-down"
                    isMask: true
                    color: Tokens.secondaryText(Kirigami.Theme)
                    rotation: accountRow.expanded ? 180 : 0

                    Behavior on rotation {
                        NumberAnimation {
                            duration: Kirigami.Units.longDuration
                            easing.type: Easing.OutCubic
                        }
                    }
                }

                background: Rectangle {
                    radius: Metrics.chipRadius(Kirigami.Units)
                    color: Tokens.chip(Kirigami.Theme)
                    opacity: expander.hovered ? 1 : 0
                }
            }
        }

        AccountConfigDetails {
            account: accountRow.account
            display: accountRow.display
            lang: accountRow.lang
            expanded: accountRow.expanded
            onLabelApplied: label => accountRow.labelApplied(label)
            onWindowToggled: (windowId, hidden) => accountRow.windowToggled(windowId, hidden)
            onRemoveConfirmed: accountRow.removeConfirmed()
        }
    }

    Rectangle {
        visible: accountRow.indicator !== ""
        x: Metrics.rowInset(Kirigami.Units)
        width: parent.width - x * 2
        height: Metrics.hairline(Kirigami.Units) * 2
        radius: height / 2
        y: accountRow.indicator === "below" ? accountRow.height - height / 2 : -height / 2
        color: Kirigami.Theme.highlightColor
    }
}
