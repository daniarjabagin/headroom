import QtQuick
import QtTest
import "../package/contents/ui/logic/Accelerator.js" as Accelerator
import "../package/contents/ui/logic/AcceleratorEncode.js" as AcceleratorEncode
import "../package/contents/ui/logic/AdvancedPrefs.js" as AdvancedPrefs
import "../package/contents/ui/logic/NotificationPrefs.js" as NotificationPrefs
import "../package/contents/ui/logic/Onboarding.js" as Onboarding
import "../package/contents/ui/logic/Options.js" as Options
import "../package/contents/ui/logic/Preferences.js" as Preferences
import "../package/contents/ui/logic/Settings.js" as Settings

TestCase {
    name: "ConfigLogic"

    function account(id, provider, providerName, extra) {
        return Object.assign({
            id,
            provider,
            providerName,
            label: null,
            email: null,
            plan: null,
            hidden: false,
            windows: []
        }, extra ?? {});
    }

    function snapshotOf(accounts) {
        return {
            accounts
        };
    }

    function test_encode_data() {
        return [
            {
                tag: "super letter",
                qt: "Meta+U",
                gtk: "<Super>u"
            },
            {
                tag: "canonical gtk order",
                qt: "Ctrl+Alt+Shift+Meta+K",
                gtk: "<Shift><Control><Alt><Super>k"
            },
            {
                tag: "function key",
                qt: "Ctrl+F12",
                gtk: "<Control>F12"
            },
            {
                tag: "named key",
                qt: "Meta+PgDown",
                gtk: "<Super>Page_Down"
            },
            {
                tag: "punctuation",
                qt: "Ctrl+,",
                gtk: "<Control>comma"
            },
            {
                tag: "plus key",
                qt: "Ctrl++",
                gtk: "<Control>plus"
            },
            {
                tag: "keypad enter",
                qt: "Meta+Enter",
                gtk: "<Super>KP_Enter"
            },
            {
                tag: "cleared",
                qt: "",
                gtk: ""
            },
            {
                tag: "multi chord",
                qt: "Ctrl+K, Ctrl+U",
                gtk: null
            },
            {
                tag: "unknown key",
                qt: "Meta+Пробел",
                gtk: null
            },
            {
                tag: "unknown modifier",
                qt: "Hyper+U",
                gtk: null
            },
            {
                tag: "prototype names are not keys",
                qt: "Meta+constructor",
                gtk: null
            },
            {
                tag: "modifier only",
                qt: "Meta+",
                gtk: null
            }
        ];
    }

    function test_encode(data) {
        compare(AcceleratorEncode.encode(data.qt), data.gtk);
    }

    function test_encode_round_trips_with_the_panel_decoder() {
        for (const sequence of ["Meta+U", "Ctrl+Alt+H", "Ctrl+Shift+Meta+F5", "Meta+PgUp", "Alt+Space", "Ctrl+Del", "Meta+Esc", "Ctrl+/", "Meta+Menu"])
            compare(Accelerator.keySequence(AcceleratorEncode.encode(sequence)), sequence);
    }

    function test_panel_limit_picker() {
        const first = {
            accountId: "claude:0a",
            window: "session"
        };
        const second = {
            accountId: "codex:1b",
            window: "weekly"
        };
        const third = {
            accountId: "codex:2c",
            window: "weekly"
        };
        const key = Options.panelLimitKey;
        compare(Preferences.toggledLimits([], key(first), true), [first]);
        compare(Preferences.toggledLimits([first, second], key(first), false), [second]);
        compare(Preferences.toggledLimits([first, second, third], "zai:3d\nsession", true), [first, second, third]);
        verify(Preferences.canChoose([first, second], key(third)));
        verify(!Preferences.canChoose([first, second, third], "zai:3d\nsession"));
        verify(Preferences.canChoose([first, second, third], key(third)));
        verify(Preferences.isChosen([first], key(first)));
        compare(Preferences.providerOf(key(second)), "codex");
        compare(Preferences.limitsSummary("en", [first, second]), "2 of 3 chosen · shown in this order");
        compare(Preferences.limitsSummary("ru", []), "Ничего не выбрано · показываются два самых критичных");
        const choices = Preferences.limitChoices("en", null, [first]);
        compare(choices, [
            {
                value: key(first),
                label: "Pinned limit (not available now)"
            }
        ]);
    }

    function test_sections_move_spend_for_new_daemons() {
        verify(Preferences.sections(false).some(section => section.key === "showSpend"));
        verify(!Preferences.sections(true).some(section => section.key === "showSpend"));
        compare(Preferences.spendPeriodOptions("en").map(option => option.value), ["today", "yesterday", "7d", "30d"]);
        compare(Preferences.reducedMotionPatch(true), {
            reduced_motion: true
        });
    }

    function test_star_rows() {
        const rows = Preferences.starRows(snapshotOf([account("claude:0a", "claude", "Claude", {
                label: "personal",
                plan: "Max 5x",
                email: "me@example.org"
            }), account("cursor:1b", "cursor", "Cursor", {
                email: "dev@example.com",
                plan: "Pro"
            }), account("zai:2c", "zai", "Z.ai", {
                hidden: true
            })]));
        compare(rows, [
            {
                id: "claude:0a",
                provider: "claude",
                title: "personal",
                subtitle: "Claude · Max 5x · me@example.org"
            },
            {
                id: "cursor:1b",
                provider: "cursor",
                title: "dev@example.com",
                subtitle: "Cursor · Pro"
            }
        ]);
        compare(Preferences.starLabel("en", true), "Always open");
        compare(Preferences.starLabel("ru", false), "По запросу");
    }

    function test_notification_rows_and_summary() {
        const rows = NotificationPrefs.providerRows(snapshotOf([account("claude:0a", "claude", "Claude"), account("claude:1b", "claude", "Claude"), account("codex:2c", "codex", "Codex")]));
        compare(rows, [
            {
                provider: "claude",
                name: "Claude"
            },
            {
                provider: "codex",
                name: "Codex"
            }
        ]);
        compare(NotificationPrefs.perProviderSummary("en", {}, rows), "Every provider uses the default");
        compare(NotificationPrefs.perProviderSummary("en", {
            claude: 20,
            copilot: 0
        }, rows), "1 provider differs from the default");
        compare(NotificationPrefs.perProviderSummary("ru", {
            claude: 20,
            codex: 0
        }, rows), "2 сервиса отличаются от общего порога");
    }

    function test_milestone_subtitle_uses_the_threshold() {
        const almostOut = Options.MILESTONES[0];
        compare(NotificationPrefs.milestoneSubtitle("en", almostOut, 20, true), "A limit drops under 20% left");
        compare(NotificationPrefs.milestoneSubtitle("en", almostOut, 20, false), "A limit drops under 10% left");
        compare(NotificationPrefs.milestoneSubtitle("en", Options.MILESTONES[1], 20, true), "The pace says a limit will barely last until reset");
    }

    function test_threshold_patches() {
        compare(NotificationPrefs.thresholdPatch(20), {
            notifications: {
                threshold_percent: 20
            }
        });
        compare(NotificationPrefs.providerPatch("claude", "default").notifications.provider_thresholds, {
            claude: null
        });
        compare(NotificationPrefs.providerPatch("copilot", 0).notifications.provider_thresholds, {
            copilot: 0
        });
        compare(NotificationPrefs.providerPatch("codex", 30).notifications.provider_thresholds, {
            codex: 30
        });
    }

    function test_quiet_hours_clock() {
        compare(NotificationPrefs.normalizedClock("7:05"), "07:05");
        compare(NotificationPrefs.normalizedClock(" 22:00 "), "22:00");
        compare(NotificationPrefs.normalizedClock("24:00"), null);
        compare(NotificationPrefs.normalizedClock("7"), null);
        const hours = Settings.fromRaw({}).notifications.quietHours;
        compare(NotificationPrefs.clockPatch(hours, "from", "23:30"), {
            notifications: {
                quiet_hours: {
                    from: "23:30"
                }
            }
        });
        compare(NotificationPrefs.clockPatch(hours, "from", "22:00"), null);
        compare(NotificationPrefs.clockPatch(hours, "to", "nope"), null);
        const enabled = Object.assign({}, hours, {
            enabled: true
        });
        compare(NotificationPrefs.clockPatch(enabled, "to", "22:00"), {
            notifications: {
                quiet_hours: {
                    enabled: false,
                    to: "22:00"
                }
            }
        });
        compare(NotificationPrefs.quietPatch({
            allowCritical: false
        }), {
            notifications: {
                quiet_hours: {
                    allow_critical: false
                }
            }
        });
    }

    function test_onboarding() {
        const fresh = Settings.fromRaw({});
        const done = Settings.fromRaw({
            onboarding: {
                completed: true
            }
        });
        verify(Onboarding.pending(fresh, true));
        verify(!Onboarding.pending(fresh, false));
        verify(!Onboarding.pending(done, true));
        verify(!Onboarding.pending(null, true));
        const found = snapshotOf([account("claude:0a", "claude", "Claude Code"), account("claude:1b", "claude", "Claude Code"), account("codex:2c", "codex", "Codex")]);
        compare(Onboarding.title("en", found), "Welcome to Headroom — we found Claude Code and Codex");
        compare(Onboarding.title("ru", found), "Добро пожаловать в Headroom — нашли Claude Code и Codex");
        compare(Onboarding.title("en", snapshotOf([account("a:1", "a", "A"), account("b:1", "b", "B"), account("c:1", "c", "C")])), "Welcome to Headroom — we found A, B and C");
        compare(Onboarding.title("en", snapshotOf([])), "Welcome to Headroom");
        compare(Onboarding.title("en", null), "Welcome to Headroom");
    }

    function test_advanced_helpers() {
        compare(AdvancedPrefs.REPOSITORY_URL, "https://github.com/daniarjabagin/headroom");
        compare(AdvancedPrefs.folderUrl("~/.local/state/headroom/headroom.log", "file:///home/ada"), "file:///home/ada/.local/state/headroom");
        compare(AdvancedPrefs.folderUrl("/var/log/my logs/headroom.log", ""), "file:///var/log/my%20logs");
        compare(AdvancedPrefs.folderUrl(null, "file:///home/ada"), "");
        compare(AdvancedPrefs.folderUrl("relative.log", "file:///home/ada"), "");
        compare(AdvancedPrefs.logLevelNote("en", {
            logLevelSource: "env"
        }), "RUST_LOG is set and wins over this setting");
        compare(AdvancedPrefs.logLevelNote("en", null), "Debug adds provider responses without tokens");
        compare(AdvancedPrefs.logFileLine("en", null, "boom"), "boom");
        compare(AdvancedPrefs.logFileLine("en", null, null), "Loading…");
        compare(AdvancedPrefs.logFileLine("en", {
            logFile: null
        }, null), "The service writes no log file");
    }
}
