pragma Singleton

import QtQuick

QtObject {
    readonly property int gridUnit: 18
    readonly property int smallSpacing: 4
    readonly property int mediumSpacing: 6
    readonly property int largeSpacing: 8
    readonly property int veryShortDuration: 50
    readonly property int shortDuration: 100
    readonly property int longDuration: 200
    readonly property int veryLongDuration: 400
    readonly property int humanMoment: 2000
    readonly property IconSizes iconSizes: IconSizes {}
}
