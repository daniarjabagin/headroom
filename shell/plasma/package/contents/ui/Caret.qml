import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

T.AbstractButton {
    id: caret

    property bool expanded: false

    Layout.fillWidth: true
    Layout.leftMargin: Kirigami.Units.largeSpacing
    Layout.rightMargin: Kirigami.Units.largeSpacing
    implicitHeight: Metrics.caretIcon(Kirigami.Units) + topPadding + bottomPadding
    topPadding: Metrics.gutter(Kirigami.Units)
    bottomPadding: Metrics.gutter(Kirigami.Units)

    contentItem: Item {
        Kirigami.Icon {
            anchors.centerIn: parent
            implicitWidth: Metrics.caretIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: "arrow-down"
            isMask: true
            color: Tokens.secondaryText(Kirigami.Theme)
            rotation: caret.expanded ? 180 : 0

            Behavior on rotation {
                NumberAnimation {
                    duration: Kirigami.Units.longDuration
                    easing.type: Easing.OutCubic
                }
            }
        }
    }

    background: HoverFill {
        radius: Metrics.chipRadius(Kirigami.Units)
        shown: pointer.shown || caret.visualFocus
    }

    PointerHover {
        id: pointer
    }
}
