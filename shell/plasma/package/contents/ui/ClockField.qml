import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

QQC2.TextField {
    id: field

    required property string clock

    signal committed(string text)

    function restore() {
        text = Qt.binding(() => field.clock);
    }

    implicitWidth: Kirigami.Units.gridUnit * 4
    horizontalAlignment: TextInput.AlignHCenter
    inputMethodHints: Qt.ImhTime
    maximumLength: 5
    text: clock
    font.features: {
        "tnum": 1
    }
    validator: RegularExpressionValidator {
        regularExpression: /^([01]?\d|2[0-3])(:[0-5]?\d?)?$/
    }
    onEditingFinished: {
        field.committed(text);
        field.restore();
    }
}
