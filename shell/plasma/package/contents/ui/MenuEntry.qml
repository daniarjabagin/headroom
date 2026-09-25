import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

QQC2.MenuItem {
    id: entry

    property string leadIcon: ""
    property string detail: ""
    property bool stacked: false
    property bool markable: false
    property bool marked: false
    readonly property var menuTheme: ({
            backgroundColor: palette.window,
            textColor: palette.windowText
        })
    readonly property color titleColor: colorFor(palette.windowText)
    readonly property color detailColor: colorFor(Tokens.secondaryText(menuTheme))

    function colorFor(normal) {
        if (!enabled)
            return Tokens.tertiaryText(menuTheme);
        return highlighted ? palette.highlightedText : normal;
    }

    implicitHeight: Math.max(implicitBackgroundHeight, implicitContentHeight + topPadding + bottomPadding)
    topPadding: stacked ? Kirigami.Units.mediumSpacing : Kirigami.Units.smallSpacing + Math.round(Kirigami.Units.smallSpacing / 2)
    bottomPadding: topPadding
    leftPadding: Kirigami.Units.largeSpacing
    rightPadding: Kirigami.Units.largeSpacing

    contentItem: RowLayout {
        spacing: Kirigami.Units.mediumSpacing + Math.round(Kirigami.Units.smallSpacing / 2)

        Kirigami.Icon {
            objectName: "menuLead"
            visible: entry.markable || entry.leadIcon !== ""
            opacity: entry.markable && !entry.marked ? 0 : 1
            Layout.alignment: entry.stacked ? Qt.AlignTop : Qt.AlignVCenter
            Layout.topMargin: entry.stacked ? Math.round(Kirigami.Units.smallSpacing / 2) : 0
            implicitWidth: Metrics.compactIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: entry.markable ? "checkmark" : entry.leadIcon
            isMask: true
            color: entry.markable ? entry.titleColor : entry.detailColor
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Math.round(Kirigami.Units.smallSpacing / 2)

            TextLabel {
                Layout.fillWidth: true
                font.features: ({})
                color: entry.titleColor
                text: entry.text
                elide: Text.ElideRight
            }

            TextLabel {
                objectName: "menuSubtitle"
                visible: entry.stacked && entry.detail !== ""
                Layout.fillWidth: true
                role: "caption"
                color: entry.detailColor
                text: entry.detail
                wrapMode: Text.Wrap
            }
        }

        TextLabel {
            objectName: "menuDetail"
            visible: !entry.stacked && entry.detail !== ""
            Layout.alignment: Qt.AlignVCenter
            role: "caption"
            font.features: ({})
            color: entry.detailColor
            text: entry.detail
        }
    }
}
