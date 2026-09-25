import QtQuick
import org.kde.kirigami as Kirigami
import "logic/Motion.js" as Motion
import "logic/Tokens.js" as Tokens

Rectangle {
    id: fill

    property bool shown: false
    property bool animated: true

    color: Tokens.hover(Kirigami.Theme)
    opacity: shown ? 1 : 0

    Behavior on opacity {
        enabled: fill.animated

        NumberAnimation {
            duration: Motion.hoverDuration(Kirigami.Units)
            easing.type: Easing.OutCubic
        }
    }
}
