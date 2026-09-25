import QtQuick
import QtQuick.Controls as QQC2

QQC2.Dialog {
    property string subtitle: ""
    property list<QtObject> customFooterActions

    modal: true
}
