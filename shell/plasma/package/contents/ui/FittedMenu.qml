import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

QQC2.Menu {
    id: menu

    property real minimumWidth: Kirigami.Units.gridUnit * 8
    property real maximumWidth: Kirigami.Units.gridUnit * 24
    readonly property real widest: {
        let result = 0;
        for (let index = 0; index < count; ++index)
            result = Math.max(result, itemAt(index)?.implicitWidth ?? 0);
        return result;
    }

    implicitWidth: Math.min(maximumWidth, Math.max(minimumWidth, Math.ceil(widest) + leftPadding + rightPadding))
    margins: Kirigami.Units.smallSpacing
}
