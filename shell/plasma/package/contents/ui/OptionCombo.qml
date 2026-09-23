import QtQuick
import QtQuick.Controls as QQC2
import "logic/Options.js" as Options

QQC2.ComboBox {
    id: combo

    property var options: []
    property var value: null

    signal picked(var value)

    model: options
    textRole: "label"
    valueRole: "value"
    currentIndex: Options.indexOfValue(options, value)
    onActivated: index => combo.picked(combo.options[index].value)
}
