pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
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

    QQC2.Menu {
        id: menu

        objectName: "unitMenu"

        Instantiator {
            model: SpendUnits.unitOptions(title.lang)

            delegate: QQC2.MenuItem {
                id: option

                required property var modelData

                objectName: `unit-${modelData.value}`
                checkable: true
                checked: modelData.value === title.unit
                text: modelData.title
                QQC2.ToolTip.text: modelData.subtitle
                QQC2.ToolTip.visible: hovered
                QQC2.ToolTip.delay: Kirigami.Units.veryLongDuration
                onTriggered: {
                    title.picked(modelData.value);
                    checked = Qt.binding(() => option.modelData.value === title.unit);
                }
            }

            onObjectAdded: (index, object) => menu.insertItem(index, object)
            onObjectRemoved: (index, object) => menu.removeItem(object)
        }
    }
}
