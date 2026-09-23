pragma Singleton

import QtQuick

QtObject {
    property string scenario: "ready"
    property string statePath: Qt.resolvedUrl("../../../../sample-state.json")
    property string metadataPath: Qt.resolvedUrl("../../../../../package/metadata.json")
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
        return json.replace(/"(\d{4}-\d\d-\d\dT[\d:.]+Z)"/g, (match, stamp) => `"${new Date(Date.parse(stamp) + offset).toISOString()}"`);
    }
}
