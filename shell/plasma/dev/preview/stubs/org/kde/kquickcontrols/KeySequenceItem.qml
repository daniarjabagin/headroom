import QtQuick
import QtQuick.Controls as QQC2

QQC2.Button {
    property var keySequence: ""
    property bool modifierlessAllowed: false
    property bool modifierOnlyAllowed: false
    property bool multiKeyShortcutsAllowed: true
    property bool showClearButton: true

    signal captureFinished

    text: String(keySequence)
}
