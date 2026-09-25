pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/SpendUnits.js" as SpendUnits
import "logic/Tokens.js" as Tokens

T.AbstractButton {
    id: title

    required property string unit
    required property string lang
    property real step: 0
    property bool animated: true

    signal picked(string key)

    objectName: "unitTitle"
    implicitWidth: row.implicitWidth + leftPadding + rightPadding
    implicitHeight: row.implicitHeight + topPadding + bottomPadding
    leftPadding: Kirigami.Units.mediumSpacing
    rightPadding: Kirigami.Units.mediumSpacing
    topPadding: Math.round(Kirigami.Units.smallSpacing / 2)
    bottomPadding: Math.round(Kirigami.Units.smallSpacing / 2)
    Layout.leftMargin: -Kirigami.Units.mediumSpacing
    hoverEnabled: true
    Accessible.role: Accessible.ButtonMenu
    Accessible.name: SpendUnits.unitTitle(lang, unit)
    onClicked: menu.popup(title, 0, title.height)

    background: HoverFill {
        animated: title.animated
        radius: Metrics.chipRadius(Kirigami.Units)
        shown: pointer.shown || title.visualFocus || menu.visible
    }

    contentItem: RowLayout {
        id: row

        spacing: Kirigami.Units.smallSpacing

        TextLabel {
            role: "title"
            step: title.step
            text: SpendUnits.unitTitle(title.lang, title.unit)
        }

        Kirigami.Icon {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Metrics.caretIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: "arrow-down"
            isMask: true
            color: Tokens.secondaryText(Kirigami.Theme)
        }
    }

    PointerHover {
        id: pointer
    }

    FittedMenu {
        id: menu

        objectName: "unitMenu"
        maximumWidth: Kirigami.Units.gridUnit * 13

        Instantiator {
            model: SpendUnits.unitOptions(title.lang)

            delegate: MenuEntry {
                required property var modelData

                objectName: `unit-${modelData.value}`
                markable: true
                marked: modelData.value === title.unit
                stacked: true
                text: modelData.title
                detail: modelData.subtitle
                onTriggered: title.picked(modelData.value)
            }

            onObjectAdded: (index, object) => menu.insertItem(index, object)
            onObjectRemoved: (index, object) => menu.removeItem(object)
        }
    }
}
