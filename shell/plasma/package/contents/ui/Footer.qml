import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import "logic/FooterStatus.js" as FooterStatus
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

PlasmaExtras.PlasmoidHeading {
    id: footer

    required property var view
    required property var now
    required property string lang
    required property string versionText
    property bool animated: true
    readonly property var status: FooterStatus.footerStatus(lang, view, now, versionText)
    readonly property real dotSize: Math.round(Kirigami.Units.smallSpacing * 1.5)
    readonly property real haloAlpha: 0.22

    signal refreshRequested
    signal settingsRequested

    function lineColor(notice, emphasis) {
        return notice ? Kirigami.Theme.neutralTextColor : emphasis === "primary" ? Kirigami.Theme.textColor : Tokens.secondaryText(Kirigami.Theme);
    }

    position: PlasmaExtras.PlasmoidHeading.Footer

    contentItem: RowLayout {
        spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing

        ColumnLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            spacing: 0

            RowLayout {
                visible: footer.status.first.text !== ""
                spacing: Kirigami.Units.smallSpacing

                Kirigami.Icon {
                    visible: footer.status.stale
                    implicitWidth: Metrics.tinyIcon(Kirigami.Units)
                    implicitHeight: implicitWidth
                    source: "dialog-warning"
                    isMask: true
                    color: Kirigami.Theme.neutralTextColor
                }

                TextLabel {
                    objectName: "footerFirst"
                    Layout.fillWidth: true
                    role: "caption"
                    color: footer.lineColor(footer.status.first.notice, "secondary")
                    text: footer.status.first.text
                    elide: Text.ElideRight
                }
            }

            T.AbstractButton {
                id: statusButton

                objectName: "footerStatus"
                visible: footer.status.second.text !== ""
                enabled: footer.view.kind === "ready"
                Layout.fillWidth: true
                Layout.maximumWidth: implicitWidth
                implicitWidth: statusRow.implicitWidth
                implicitHeight: statusRow.implicitHeight
                Accessible.role: Accessible.Button
                Accessible.name: footer.status.second.text
                Accessible.description: I18n.tr(footer.lang, "Refresh")
                onClicked: footer.refreshRequested()

                PointerHover {
                    id: pointer
                }

                HoverTip {
                    text: footer.status.tip
                }

                contentItem: RowLayout {
                    id: statusRow

                    spacing: Kirigami.Units.smallSpacing

                    Rectangle {
                        objectName: "liveDot"
                        visible: footer.status.live
                        Layout.alignment: Qt.AlignVCenter
                        Layout.leftMargin: Metrics.hairline(Kirigami.Units)
                        implicitWidth: footer.dotSize + Math.round(Kirigami.Units.smallSpacing * 1.5)
                        implicitHeight: implicitWidth
                        radius: width / 2
                        color: Tokens.alpha(Kirigami.Theme.positiveTextColor, footer.haloAlpha)

                        Rectangle {
                            anchors.centerIn: parent
                            width: footer.dotSize
                            height: width
                            radius: width / 2
                            color: Kirigami.Theme.positiveTextColor
                        }
                    }

                    TextLabel {
                        id: statusText

                        Layout.fillWidth: true
                        role: "caption"
                        color: footer.lineColor(footer.status.second.notice, pointer.shown ? "primary" : "secondary")
                        text: footer.status.second.text
                        elide: Text.ElideRight
                    }

                    PlasmaComponents3.BusyIndicator {
                        visible: footer.status.busy
                        running: visible && footer.animated
                        Layout.preferredWidth: Kirigami.Units.iconSizes.small * 0.75
                        Layout.preferredHeight: Kirigami.Units.iconSizes.small * 0.75
                    }
                }
            }
        }

        IconButton {
            objectName: "settingsButton"
            Layout.alignment: Qt.AlignVCenter
            iconName: "configure"
            animated: footer.animated
            text: I18n.tr(footer.lang, "Settings")
            tipText: `${text} · ${footer.versionText}`
            onClicked: footer.settingsRequested()
        }
    }
}
