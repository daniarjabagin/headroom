import QtQuick
import headroom.preview

QtObject {
    property int busType: 0
    property string watchedService: ""
    readonly property bool registered: PreviewConfig.scenario !== "unavailable"
}
