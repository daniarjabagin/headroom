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
    property var statePatch: ({})
    property var checkReplies: []
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
        if (scenario === "retrying-error")
            shifted.accounts.filter(account => account.status === "error").forEach(account => account.status = "refreshing");
        Object.assign(shifted, statePatch);
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
            command: install === "package" ? "Download the new Arch package from https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0 and install it with sudo pacman -U" : "headroom update"
        };
    }

    function nextCheckReply() {
        const [reply, ...rest] = checkReplies;
        checkReplies = rest;
        return reply ?? {
            value: JSON.stringify({
                status: "up_to_date",
                checked_at: new Date().toISOString(),
                version: "0.6.0"
            })
        };
    }

    function providersJson() {
        return readFile(providersPath);
    }

    function baseSettings() {
        return {
            refresh_interval_secs: 300,
            adaptive_refresh: true,
            notifications: {
                almost_out: true,
                cutting_it_close: true,
                will_run_out: true,
                reset: false,
                threshold_percent: 10,
                provider_thresholds: {},
                quiet_hours: {
                    enabled: false,
                    from: "22:00",
                    to: "08:00",
                    allow_critical: true
                }
            },
            headline: {
                mode: "auto"
            },
            reduced_motion: false,
            updates: {
                check: true
            },
            status_pages: {
                enabled: false
            },
            shortcuts: {
                open: ""
            },
            logging: {
                level: "info"
            },
            onboarding: {
                completed: true
            },
            display: JSON.parse(shiftedState()).display
        };
    }

    function defaultDisplay() {
        return {
            theme: "system",
            language: "system",
            value_mode: "left",
            reset_format: "countdown",
            panel_label: "percent",
            show_spend: true,
            show_account_spend: true,
            show_trend: true,
            show_forecast: true,
            translucent: false,
            combine_accounts: false,
            hidden_windows: {},
            density: "normal",
            time_format: "auto",
            panel_mode: "headline",
            panel_indicator: "ring",
            panel_limits: [],
            panel_position: {
                box: "right",
                index: 0
            },
            spend_period: "30d",
            spend_unit: "cost",
            spend_breakdown: "models",
            starred_accounts: [],
            collapse_unstarred: false,
            hide_on_screen_share: true
        };
    }

    function resetSettings() {
        const onboarding = currentSettings().onboarding;
        appliedSettings = Object.assign(baseSettings(), {
            display: defaultDisplay(),
            onboarding
        });
    }

    function spendRow(key, provider, cost, tokens, share) {
        return {
            key,
            provider,
            tokens: {
                input: Math.round(tokens * 0.2),
                cache_read: tokens - Math.round(tokens * 0.2) - Math.round(tokens * 0.05),
                cache_write: 0,
                output: Math.round(tokens * 0.05),
                reasoning: 0,
                total: tokens
            },
            cost_usd_micros: cost,
            partial: false,
            unpriced_tokens: 0,
            cost_per_mtok_usd_micros: tokens === 0 ? null : Math.round(cost * 1000000 / tokens),
            share_permille: share
        };
    }

    function spendJson(queryJson) {
        const query = JSON.parse(queryJson);
        const period = JSON.parse(shiftedState()).spend.last_7_days;
        const rows = period.by_provider.map(entry => spendRow(entry.provider, entry.provider, entry.cost_usd_micros, entry.total_tokens, Math.floor(entry.cost_usd_micros * 1000 / period.cost_usd_micros)));
        const today = new Date().toISOString().slice(0, 10);
        return JSON.stringify({
            since: query.since ?? new Date(Date.now() - 6 * 86400000).toISOString().slice(0, 10),
            until: query.until ?? today,
            by: query.by,
            rows,
            total: spendRow(null, null, period.cost_usd_micros, period.total_tokens, 1000)
        });
    }

    function diagnosticsJson() {
        const text = "Headroom 0.6.0\nOS: Arch Linux\nDesktop: KDE (wayland)\nUptime: 2h 5m\nIPC: dbus\nLog level: info (settings)\nLog file: ~/.local/state/headroom/headroom.log\n";
        return JSON.stringify({
            app_version: "0.6.0",
            os: "Arch Linux",
            desktop: "KDE (wayland)",
            uptime_secs: 7530,
            transports: ["dbus"],
            log_level: currentSettings().logging?.level ?? "info",
            log_level_source: "settings",
            log_file: "~/.local/state/headroom/headroom.log",
            providers: [],
            accounts: [],
            text
        });
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
