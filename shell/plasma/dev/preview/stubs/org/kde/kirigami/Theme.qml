pragma Singleton

import QtQuick

QtObject {
    property bool dark: false
    readonly property color backgroundColor: dark ? "#202326" : "#eff0f1"
    readonly property color alternateBackgroundColor: dark ? "#292c30" : "#e3e5e7"
    readonly property color textColor: dark ? "#fcfcfc" : "#232629"
    readonly property color disabledTextColor: dark ? "#6e7174" : "#a0a1a3"
    readonly property color highlightColor: "#3daee9"
    readonly property color highlightedTextColor: "#ffffff"
    readonly property color linkColor: dark ? "#1d99f3" : "#2980b9"
    readonly property color positiveTextColor: "#27ae60"
    readonly property color neutralTextColor: "#f67400"
    readonly property color negativeTextColor: "#da4453"
    readonly property font defaultFont: Qt.font({
        family: "Noto Sans",
        pointSize: 10
    })
    readonly property font smallFont: Qt.font({
        family: "Noto Sans",
        pointSize: 8
    })
}
