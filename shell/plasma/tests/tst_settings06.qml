import QtQuick
import QtTest
import "../package/contents/ui/logic/Compat.js" as Compat
import "../package/contents/ui/logic/Options.js" as Options
import "../package/contents/ui/logic/Settings.js" as Settings
import "../package/contents/ui/logic/SettingsValues.js" as Values

TestCase {
    name: "Settings06"

    readonly property var fullDocument: ({
            refresh_interval_secs: 300,
            adaptive_refresh: false,
            notifications: {
                almost_out: true,
                cutting_it_close: true,
                will_run_out: true,
                reset: false,
                threshold_percent: 20,
                provider_thresholds: {
                    claude: 30,
                    copilot: 0
                },
                quiet_hours: {
                    enabled: true,
                    from: "23:30",
                    to: "07:15",
                    allow_critical: false
                }
            },
            headline: {
                mode: "auto"
            },
            reduced_motion: false,
            display: {
                density: "compact",
                time_format: "12h",
                panel_mode: "several",
                panel_indicator: "bar",
                panel_label: "none",
                panel_limits: [
                    {
                        account_id: "claude:9f8e",
                        window: "session"
                    },
                    {
                        account_id: "codex:1a2b",
                        window: "weekly"
                    }
                ],
                panel_position: {
                    box: "left",
                    index: 2
                },
                spend_period: "7d",
                spend_unit: "cost_per_mtok",
                spend_breakdown: "projects",
                starred_accounts: ["claude:9f8e"],
                collapse_unstarred: true,
                hide_on_screen_share: false
            },
            updates: {
                check: true
            },
            status_pages: {
                enabled: true
            },
            shortcuts: {
                open: "<Super>u"
            },
            logging: {
                level: "debug"
            },
            onboarding: {
                completed: true
            }
        })

    function parse(raw) {
        return Settings.fromRaw(Settings.decode(JSON.stringify(raw)));
    }

    function test_defaults_when_keys_are_missing() {
        const settings = parse({});
        compare(settings.adaptiveRefresh, true);
        compare(settings.statusPages, {
            enabled: false
        });
        compare(settings.shortcuts, {
            open: ""
        });
        compare(settings.logging, {
            level: "info"
        });
        compare(settings.onboarding, {
            completed: false
        });
        compare(settings.notifications.thresholdPercent, 10);
        compare(settings.notifications.providerThresholds, {});
        compare(settings.notifications.quietHours, {
            enabled: false,
            from: "22:00",
            to: "08:00",
            allowCritical: true
        });
        const display = settings.display;
        compare([display.density, display.timeFormat, display.panelMode, display.panelIndicator, display.panelLabel], ["normal", "auto", "headline", "ring", "percent"]);
        compare(display.panelLimits, []);
        compare(display.panelPosition, {
            box: "right",
            index: 0
        });
        compare([display.spendPeriod, display.spendUnit, display.spendBreakdown], ["30d", "cost", "models"]);
        compare(display.starredAccounts, []);
        compare(display.collapseUnstarred, false);
        compare(display.hideOnScreenShare, true);
    }

    function test_reads_every_new_key() {
        const settings = parse(fullDocument);
        compare(settings.adaptiveRefresh, false);
        verify(settings.statusPages.enabled);
        compare(settings.shortcuts.open, "<Super>u");
        compare(settings.logging.level, "debug");
        verify(settings.onboarding.completed);
        compare(settings.notifications.thresholdPercent, 20);
        compare(settings.notifications.providerThresholds, {
            claude: 30,
            copilot: 0
        });
        compare(settings.notifications.quietHours, {
            enabled: true,
            from: "23:30",
            to: "07:15",
            allowCritical: false
        });
        const display = settings.display;
        compare([display.density, display.timeFormat, display.panelMode, display.panelIndicator, display.panelLabel], ["compact", "12h", "several", "bar", "none"]);
        compare(display.panelLimits, [
            {
                accountId: "claude:9f8e",
                window: "session"
            },
            {
                accountId: "codex:1a2b",
                window: "weekly"
            }
        ]);
        compare(display.panelPosition, {
            box: "left",
            index: 2
        });
        compare([display.spendPeriod, display.spendUnit, display.spendBreakdown], ["7d", "cost_per_mtok", "projects"]);
        compare(display.starredAccounts, ["claude:9f8e"]);
        verify(display.collapseUnstarred);
        verify(!display.hideOnScreenShare);
    }

    function test_invalid_values_fall_back_like_the_daemon() {
        const settings = parse({
            adaptive_refresh: "yes",
            status_pages: true,
            shortcuts: {
                open: "Super+U"
            },
            logging: {
                level: "trace"
            },
            notifications: {
                threshold_percent: 90,
                provider_thresholds: {
                    claude: 51,
                    codex: 10.5,
                    copilot: 0,
                    "": 5,
                    grok: 12
                },
                quiet_hours: {
                    enabled: 1,
                    from: "24:00",
                    to: "7:00"
                }
            },
            display: {
                density: "tight",
                time_format: "military",
                panel_mode: "all",
                panel_indicator: "dot",
                panel_label: "icon",
                panel_limits: [
                    {
                        account_id: "a",
                        window: "session"
                    },
                    {
                        account_id: "a",
                        window: "session"
                    },
                    {
                        account_id: "",
                        window: "weekly"
                    },
                    "b",
                    {
                        account_id: "b",
                        window: "weekly"
                    },
                    {
                        account_id: "c",
                        window: "weekly"
                    },
                    {
                        account_id: "d",
                        window: "weekly"
                    }
                ],
                panel_position: {
                    box: "middle",
                    index: 1
                },
                starred_accounts: ["a", "", "a", 3, "b"]
            }
        });
        compare(settings.adaptiveRefresh, true);
        compare(settings.statusPages.enabled, false);
        compare(settings.shortcuts.open, "");
        compare(settings.logging.level, "info");
        compare(settings.notifications.thresholdPercent, 50);
        compare(settings.notifications.providerThresholds, {
            copilot: 0,
            grok: 12
        });
        compare(settings.notifications.quietHours, {
            enabled: false,
            from: "22:00",
            to: "08:00",
            allowCritical: true
        });
        const display = settings.display;
        compare([display.density, display.timeFormat, display.panelMode, display.panelIndicator, display.panelLabel], ["normal", "auto", "headline", "ring", "percent"]);
        compare(display.panelLimits.map(limit => limit.accountId), ["a", "b", "c"]);
        compare(display.panelPosition, {
            box: "right",
            index: 0
        });
        compare(display.starredAccounts, ["a", "b"]);
        compare(parse({
            notifications: {
                threshold_percent: 0
            }
        }).notifications.thresholdPercent, 1);
    }

    function test_value_validators() {
        verify(Values.isClock("00:00"));
        verify(Values.isClock("23:59"));
        verify(!Values.isClock("24:00"));
        verify(!Values.isClock("9:00"));
        verify(Values.isShortcut(""));
        verify(Values.isShortcut("<Control><Alt>h"));
        verify(Values.isShortcut("<Super>F12"));
        verify(!Values.isShortcut("<Super>"));
        verify(!Values.isShortcut("<Super> u"));
        verify(!Values.isShortcut(`<Super>${"a".repeat(64)}`));
        verify(Values.canEnableQuietHours({
            from: "22:00",
            to: "08:00"
        }));
        verify(!Values.canEnableQuietHours({
            from: "08:00",
            to: "08:00"
        }));
    }

    function test_display_patches_encode_new_keys() {
        compare(Settings.displayPatch({
            density: "compact",
            timeFormat: "24h",
            panelMode: "several",
            panelIndicator: "bar",
            panelLabel: "none",
            panelLimits: [
                {
                    accountId: "claude:9f8e",
                    window: "session"
                },
                {
                    accountId: "claude:9f8e",
                    window: "session"
                },
                {
                    accountId: "",
                    window: "weekly"
                }
            ],
            panelPosition: {
                box: "center",
                index: 1
            },
            spendPeriod: "7d",
            spendUnit: "tokens",
            spendBreakdown: "projects",
            starredAccounts: ["a", "a", "b"],
            collapseUnstarred: true,
            hideOnScreenShare: false
        }), {
            display: {
                density: "compact",
                time_format: "24h",
                panel_mode: "several",
                panel_indicator: "bar",
                panel_label: "none",
                panel_limits: [
                    {
                        account_id: "claude:9f8e",
                        window: "session"
                    }
                ],
                panel_position: {
                    box: "center",
                    index: 1
                },
                spend_period: "7d",
                spend_unit: "tokens",
                spend_breakdown: "projects",
                starred_accounts: ["a", "b"],
                collapse_unstarred: true,
                hide_on_screen_share: false
            }
        });
        compare(Settings.displayPatch({
            panelLimits: null,
            panelPosition: null
        }), {
            display: {
                panel_limits: null,
                panel_position: null
            }
        });
    }

    function test_notification_and_top_level_patches() {
        compare(Settings.notificationsPatch({
            thresholdPercent: 70,
            quietHours: {
                enabled: true,
                from: "21:00",
                to: "bad"
            }
        }), {
            notifications: {
                threshold_percent: 50,
                quiet_hours: {
                    enabled: true,
                    from: "21:00"
                }
            }
        });
        compare(Settings.notificationsPatch({
            quietHours: {
                allowCritical: false
            }
        }), {
            notifications: {
                quiet_hours: {
                    allow_critical: false
                }
            }
        });
        compare(Settings.providerThresholdPatch("claude", 20), {
            notifications: {
                provider_thresholds: {
                    claude: 20
                }
            }
        });
        compare(Settings.providerThresholdPatch("copilot", 0).notifications.provider_thresholds.copilot, 0);
        compare(Settings.providerThresholdPatch("copilot", 80).notifications.provider_thresholds.copilot, 50);
        compare(Settings.providerThresholdPatch("claude", null).notifications.provider_thresholds.claude, null);
        compare(Settings.adaptiveRefreshPatch(false), {
            adaptive_refresh: false
        });
        compare(Settings.statusPagesPatch(true), {
            status_pages: {
                enabled: true
            }
        });
        compare(Settings.shortcutPatch("<Super>u"), {
            shortcuts: {
                open: "<Super>u"
            }
        });
        compare(Settings.shortcutPatch("nonsense key").shortcuts.open, "");
        compare(Settings.loggingPatch("warn"), {
            logging: {
                level: "warn"
            }
        });
        compare(Settings.loggingPatch("trace").logging.level, "info");
        compare(Settings.onboardingPatch(true), {
            onboarding: {
                completed: true
            }
        });
    }

    function test_provider_threshold_merge_patch() {
        const raw = Settings.mergePatch(fullDocument, Settings.providerThresholdPatch("claude", null));
        compare(Settings.fromRaw(raw).notifications.providerThresholds, {
            copilot: 0
        });
    }

    function test_starred_patches() {
        const display = Settings.parseDisplay(fullDocument.display);
        verify(Settings.isStarred(display, "claude:9f8e"));
        verify(!Settings.isStarred(display, "codex:1a2b"));
        compare(Settings.starredPatch(display, "codex:1a2b", true), {
            starredAccounts: ["claude:9f8e", "codex:1a2b"]
        });
        compare(Settings.starredPatch(display, "claude:9f8e", false), {
            starredAccounts: []
        });
    }

    function test_patches_pass_through_to_capable_daemons() {
        const patch = {
            display: {
                density: "compact"
            }
        };
        compare(Compat.compatiblePatch(patch, true), patch);
        compare(Compat.compatiblePatch({}, true), null);
        compare(Compat.compatiblePatch(null, true), null);
    }

    function test_new_keys_are_stripped_for_older_daemons() {
        compare(Compat.compatiblePatch({
            adaptive_refresh: false,
            refresh_interval_secs: 600,
            status_pages: {
                enabled: true
            },
            notifications: {
                reset: true,
                threshold_percent: 20,
                quiet_hours: null
            },
            display: {
                theme: "dark",
                density: "compact",
                panel_label: "none",
                panel_limits: null
            }
        }, false), {
            refresh_interval_secs: 600,
            notifications: {
                reset: true
            },
            display: {
                theme: "dark"
            }
        });
        compare(Compat.compatiblePatch({
            display: {
                panel_label: "window"
            }
        }, false), {
            display: {
                panel_label: "window"
            }
        });
        compare(Compat.compatiblePatch(Settings.displayPatch({
            timeFormat: "12h"
        }), false), null);
        compare(Compat.compatiblePatch({
            logging: {
                level: "debug"
            },
            onboarding: {
                completed: true
            }
        }, false), null);
        compare(Compat.compatiblePatch({
            display: {
                hidden_windows: {
                    "codex:1a2b": null
                }
            }
        }, false), {
            display: {
                hidden_windows: {
                    "codex:1a2b": null
                }
            }
        });
        verify(!Compat.supports06(null));
    }

    function test_option_lists() {
        compare(Options.densityOptions("ru").map(option => option.label), ["Обычная", "Компактная"]);
        compare(Options.timeFormatOptions("en").map(option => option.value), ["auto", "12h", "24h"]);
        compare(Options.timeFormatOptions("en").map(option => option.label), ["Automatic", "12-hour", "24-hour"]);
        compare(Options.panelModeOptions("en").map(option => option.value), ["headline", "several", "icon"]);
        compare(Options.panelIndicatorOptions("ru").map(option => option.label), ["Кольцо", "Полоса", "Нет"]);
        compare(Options.panelLabelOptions("en").map(option => option.value), ["percent", "window"]);
        compare(Options.panelLabelOptions("en", true).map(option => option.value), ["percent", "window", "none"]);
        compare(Options.spendUnitOptions("en").map(option => option.value), ["cost", "tokens", "cost_per_mtok"]);
        compare(Options.spendBreakdownOptions("ru").map(option => option.label), ["Модели", "Проекты"]);
        compare(Options.logLevelOptions("en").map(option => option.value), ["error", "warn", "info", "debug"]);
        compare(Options.thresholdOptions("en", 10).map(option => option.label), ["5%", "10%", "20%", "30%"]);
        compare(Options.thresholdOptions("en", 15).map(option => option.value), [5, 10, 15, 20, 30]);
    }

    function test_provider_threshold_options() {
        const options = Options.providerThresholdOptions("en", 10, undefined);
        compare(options.map(option => option.value), ["default", 0, 5, 10, 20, 30]);
        compare(options[0].label, "Default (10%)");
        compare(options[1].label, "Off");
        compare(Options.providerThresholdOptions("ru", 20, 25).map(option => option.value), ["default", 0, 5, 10, 20, 25, 30]);
        compare(Options.providerThresholdOptions("ru", 20, 0)[0].label, "По умолчанию (20%)");
        const thresholds = {
            copilot: 0
        };
        compare(Options.providerThresholdKey(thresholds, "copilot"), 0);
        compare(Options.providerThresholdKey(thresholds, "claude"), "default");
        compare(Options.providerThresholdFor("default"), null);
        compare(Options.providerThresholdFor(20), 20);
    }

    function test_panel_limit_options() {
        const state = {
            accounts: [
                {
                    id: "claude:9f8e",
                    provider: "claude",
                    providerName: "Claude",
                    label: null,
                    email: null,
                    hidden: false,
                    windows: [
                        {
                            id: "session",
                            label: "Session"
                        }
                    ]
                }
            ]
        };
        const limit = {
            accountId: "gone:1",
            window: "weekly"
        };
        const options = Options.panelLimitOptions("en", state, limit);
        compare(options.map(option => option.label), ["Claude — Session", "Pinned limit (not available now)"]);
        compare(Options.panelLimitFor(options[0].value), {
            accountId: "claude:9f8e",
            window: "session"
        });
        compare(Options.panelLimitFor(Options.panelLimitKey(limit)), limit);
        compare(Options.panelLimitOptions("en", null, null), []);
    }
}
