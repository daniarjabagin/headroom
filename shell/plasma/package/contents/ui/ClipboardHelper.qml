import QtQuick

TextEdit {
    function copyText(value) {
        text = value;
        selectAll();
        copy();
        text = "";
    }

    visible: false
}
