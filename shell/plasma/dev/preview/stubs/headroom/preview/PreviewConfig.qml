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
    readonly property string updatePrefix: "update-"

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
        if (scenario === "retrying")
            shifted.accounts.filter(account => account.status === "signed_out").forEach(account => account.status = "refreshing");
        if (scenario === "single-spend")
            keepFirstSpender(shifted.spend);
        if (scenario.startsWith(updatePrefix))
            shifted.update = availableUpdate(scenario.slice(updatePrefix.length));
        if (appliedSettings?.updates?.check === false)
            shifted.update = null;
        return JSON.stringify(shifted);
    }

    function keepFirstSpender(spend) {
        for (const period of Object.values(spend).filter(value => value?.by_provider !== undefined)) {
            const first = period.by_provider[0];
            period.by_provider = [first];
            period.cost_usd_micros = first.cost_usd_micros;
            period.total_tokens = first.total_tokens;
            period.partial = first.partial;
        }
    }

    function availableUpdate(install) {
        return {
            version: "0.5.0",
            url: "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0",
            published_at: new Date(Date.now() - 2 * 86400000).toISOString(),
            install,
            command: install === "package" ? "sudo pacman -Syu headroom" : ""
        };
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
            updates: {
                check: true
            },
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
