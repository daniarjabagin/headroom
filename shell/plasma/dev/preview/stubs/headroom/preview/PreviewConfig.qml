pragma Singleton

import QtQuick

QtObject {
    property string scenario: "ready"
    property string statePath: Qt.resolvedUrl("../../../../sample-state.json")
    property string metadataPath: Qt.resolvedUrl("../../../../../package/metadata.json")
    property var displayPatch: ({})
    property bool wallpaper: false
    readonly property int dialogPadding: 8
    readonly property int dialogRadius: 8
    readonly property int clockCushionMs: 30000

    function readFile(url) {
        const request = new XMLHttpRequest();
        request.open("GET", url, false);
        request.send();
        return request.responseText;
    }

    function shiftedState() {
        const json = readFile(statePath);
        const generated = Date.parse(JSON.parse(json).generated_at);
        const offset = Date.now() - generated + clockCushionMs;
        const shifted = JSON.parse(json.replace(/"(\d{4}-\d\d-\d\dT[\d:.]+Z)"/g, (match, stamp) => `"${new Date(Date.parse(stamp) + offset).toISOString()}"`));
        shifted.display = Object.assign({}, shifted.display ?? {}, displayPatch);
        return JSON.stringify(shifted);
    }

    function settingsJson() {
        return JSON.stringify({
            refresh_interval_secs: 300,
            notifications: {
                almost_out: true,
                cutting_it_close: true,
                will_run_out: true,
                reset: false
            },
            headline: {
                mode: "auto"
            },
            reduced_motion: false,
            display: JSON.parse(shiftedState()).display
        });
    }
}
