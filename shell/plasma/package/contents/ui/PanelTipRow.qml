import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/PanelLayout.js" as PanelLayout
import "logic/Tokens.js" as Tokens

GridLayout {
    id: tipRow

    required property var row

    columns: 3
    columnSpacing: Kirigami.Units.largeSpacing
    rowSpacing: 0

    ProviderIcon {
        Layout.alignment: Qt.AlignVCenter
        provider: tipRow.row.provider
        color: Kirigami.Theme.textColor
    }

    PlasmaComponents3.Label {
        Layout.fillWidth: true
        textFormat: Text.PlainText
        text: tipRow.row.title
    }

    PlasmaComponents3.Label {
        Layout.alignment: Qt.AlignRight
        textFormat: Text.PlainText
        text: tipRow.row.reading
        color: PanelLayout.toneColor(Kirigami.Theme, tipRow.row.tone)
        font.weight: Font.DemiBold
        font.features: {
            "tnum": 1
        }
    }

    PlasmaComponents3.Label {
        visible: tipRow.row.detail !== ""
        Layout.column: 1
        Layout.row: 1
        Layout.columnSpan: 2
        textFormat: Text.PlainText
        text: tipRow.row.detail
        color: Tokens.secondaryText(Kirigami.Theme)
        font.features: {
            "tnum": 1
        }
    }
}
