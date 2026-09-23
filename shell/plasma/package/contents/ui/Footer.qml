import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import "logic/I18n.js" as I18n
import "logic/Summary.js" as Summary

PlasmaExtras.PlasmoidHeading {
    id: footer

    required property var view
    required property var now
    required property string lang
    required property string versionText
    readonly property var status: Summary.footerLine(lang, view, now)

    signal refreshRequested
    signal settingsRequested

    position: PlasmaExtras.PlasmoidHeading.Footer

    contentItem: RowLayout {
        spacing: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing

        ColumnLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            spacing: 0

            TextLabel {
                Layout.fillWidth: true
                role: "caption"
                emphasis: "secondary"
                text: footer.versionText
            }

            T.AbstractButton {
                id: statusButton

                objectName: "footerStatus"
                visible: footer.status.text !== ""
                enabled: footer.view.kind === "ready"
                implicitWidth: statusRow.implicitWidth
                implicitHeight: statusRow.implicitHeight
                Accessible.role: Accessible.Button
                Accessible.name: footer.status.text
                Accessible.description: I18n.tr(footer.lang, "Refresh")
                onClicked: footer.refreshRequested()

                contentItem: RowLayout {
                    id: statusRow

                    spacing: Kirigami.Units.smallSpacing

                    TextLabel {
                        id: statusText

                        role: "caption"
                        emphasis: statusButton.hovered ? "primary" : "secondary"
                        color: footer.status.notice ? Kirigami.Theme.neutralTextColor : statusText.colorFor(statusText.emphasis)
                        text: footer.status.text
                    }

                    PlasmaComponents3.BusyIndicator {
                        visible: footer.status.busy
                        running: visible
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
            text: I18n.tr(footer.lang, "Settings")
            onClicked: footer.settingsRequested()
        }
    }
}
