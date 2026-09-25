import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

RowLayout {
    id: divider

    required property string lang

    signal collapseRequested

    objectName: "foldDivider"
    Layout.fillWidth: true
    Layout.leftMargin: Metrics.headerInset(Kirigami.Units)
    spacing: Kirigami.Units.largeSpacing

    TextLabel {
        role: "caption"
        emphasis: "secondary"
        weight: Font.DemiBold
        text: I18n.tr(divider.lang, "Not pinned")
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignVCenter
        implicitHeight: Metrics.hairline(Kirigami.Units)
        color: Tokens.separator(Kirigami.Theme)
    }

    T.AbstractButton {
        id: less

        objectName: "showLess"
        implicitWidth: lessRow.implicitWidth + leftPadding + rightPadding
        implicitHeight: lessRow.implicitHeight + topPadding + bottomPadding
        leftPadding: Kirigami.Units.mediumSpacing
        rightPadding: Kirigami.Units.mediumSpacing
        topPadding: Math.round(Kirigami.Units.smallSpacing / 2)
        bottomPadding: Math.round(Kirigami.Units.smallSpacing / 2)
        hoverEnabled: true
        Accessible.role: Accessible.Button
        Accessible.name: I18n.tr(divider.lang, "Show less")
        onClicked: divider.collapseRequested()

        background: HoverFill {
            radius: Metrics.chipRadius(Kirigami.Units)
            shown: pointer.shown || less.visualFocus
        }

        contentItem: RowLayout {
            id: lessRow

            spacing: Kirigami.Units.smallSpacing

            TextLabel {
                role: "caption"
                emphasis: pointer.shown ? "primary" : "secondary"
                weight: Font.Medium
                text: I18n.tr(divider.lang, "Show less")
            }

            Kirigami.Icon {
                implicitWidth: Metrics.caretIcon(Kirigami.Units)
                implicitHeight: implicitWidth
                source: "arrow-up"
                isMask: true
                color: Tokens.secondaryText(Kirigami.Theme)
            }
        }

        PointerHover {
            id: pointer
        }
    }
}
