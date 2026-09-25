import QtQuick
import QtQuick.Controls as QQC2
import "logic/Options.js" as Options

QQC2.ComboBox {
    id: combo

    property var options: []
    property var value: null
    property var entries: []

    signal picked(var value)

    function sameEntries(left, right) {
        return left.length === right.length && left.every((entry, index) => entry.value === right[index].value && entry.label === right[index].label);
    }

    function adoptOptions() {
        if (!sameEntries(entries, options))
            entries = options;
        selectValue();
    }

    function selectValue() {
        const index = Options.indexOfValue(entries, value);
        if (currentIndex !== index)
            currentIndex = index;
    }

    model: entries
    textRole: "label"
    valueRole: "value"
    onOptionsChanged: {
        if (!sameEntries(entries, options))
            Qt.callLater(combo.adoptOptions);
    }
    onValueChanged: selectValue()
    onActivated: index => combo.picked(combo.entries[index].value)
    Component.onCompleted: adoptOptions()
}
