pragma Singleton

import QtQuick
import "../../../../../package/contents/ui/logic/Settings.js" as Settings

QtObject {
    property string scenario: "ready"
    property string statePath: Qt.resolvedUrl("../../../../sample-state.json")
    property string providersPath: Qt.resolvedUrl("../../../../sample-providers.json")
    property string metadataPath: Qt.resolvedUrl("../../../../../package/metadata.json")
    property var displayPatch: ({})
    property var appliedSettings: null
    property var failingMembers: []
    property bool wallpaper: false
    readonly property int dialogPadding: 8
    readonly property int dialogRadius: 8
    readonly property int clockCushionMs: 30000
    readonly property var refreshingFrom: ["fresh", "stale"]

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
        shifted.display = appliedSettings?.display ?? Object.assign({}, shifted.display ?? {}, displayPatch);
        if (scenario === "refreshing")
            shifted.accounts.filter(account => refreshingFrom.includes(account.status)).forEach(account => account.status = "refreshing");
        return JSON.stringify(shifted);
    }

    function providersJson() {
        return readFile(providersPath);
    }

    function baseSettings() {
        return {
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
        };
    }

    function currentSettings() {
        return appliedSettings ?? baseSettings();
    }

    function applySettingsPatch(json) {
        appliedSettings = Settings.mergePatch(currentSettings(), JSON.parse(json));
    }

    function settingsJson() {
        return JSON.stringify(currentSettings());
    }
}
