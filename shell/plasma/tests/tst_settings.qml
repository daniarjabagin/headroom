import QtQuick
import QtTest
import "../package/contents/ui/logic/Commands.js" as Commands
import "../package/contents/ui/logic/Motion.js" as Motion
import "../package/contents/ui/logic/Options.js" as Options
import "../package/contents/ui/logic/Order.js" as Order
import "../package/contents/ui/logic/PatchQueue.js" as PatchQueue
import "../package/contents/ui/logic/Settings.js" as Settings
import "../package/contents/ui/logic/Tokens.js" as Tokens

TestCase {
    readonly property string settingsJson: JSON.stringify({
        refresh_interval_secs: 600,
        notifications: {
            almost_out: false,
            cutting_it_close: true,
            will_run_out: true,
            reset: true
        },
        headline: {
            mode: "pinned",
            account_id: "codex:1a2b3c4d5e6f",
            window: "weekly"
        },
        reduced_motion: false,
        display: {
            theme: "dark",
            language: "ru",
            value_mode: "used",
            reset_format: "exact",
            panel_label: "window",
            show_spend: false,
            show_account_spend: true,
            show_trend: true,
            show_forecast: false,
            hidden_windows: {
                "codex:1a2b3c4d5e6f": ["weekly"]
            }
        }
    })

    function parse(json) {
        return Settings.fromRaw(Settings.decode(json));
    }

    function test_parse_settings() {
        const settings = parse(settingsJson);
        compare(settings.refreshIntervalSecs, 600);
        compare(settings.notifications.almostOut, false);
        compare(settings.headline, {
            mode: "pinned",
            accountId: "codex:1a2b3c4d5e6f",
            window: "weekly"
        });
        compare(settings.display.theme, "dark");
        compare(settings.display.panelLabel, "window");
        compare(settings.display.translucent, false);
        compare(settings.reducedMotion, false);
        verify(parse("{\"reduced_motion\": true}").reducedMotion);
        compare(parse("{\"refresh_interval_secs\": 5}").refreshIntervalSecs, 60);
        compare(Settings.parseDisplay({
            theme: "neon",
            translucent: true
        }).theme, "system");
        verify(Settings.parseDisplay({
            translucent: true
        }).translucent);
    }

    function test_bad_settings() {
        for (const json of ["{", "[]", "null"]) {
            try {
                parse(json);
                fail(`expected a settings error for ${json}`);
            } catch (error) {
                verify(Settings.isSettingsError(error));
            }
        }
    }

    function test_merge_patches() {
        compare(Settings.displayPatch({
            valueMode: "left",
            translucent: true,
            bogus: 1
        }), {
            display: {
                value_mode: "left",
                translucent: true
            }
        });
        compare(Settings.notificationsPatch({
            reset: false
        }), {
            notifications: {
                reset: false
            }
        });
        compare(Settings.refreshIntervalPatch(90000), {
            refresh_interval_secs: 3600
        });
        compare(Settings.headlinePatch({
            mode: "auto"
        }), {
            headline: {
                mode: "auto",
                account_id: null,
                window: null
            }
        });
        compare(Settings.headlinePatch({
            mode: "pinned",
            accountId: "claude:0a1b2c3d4e5f",
            window: "session"
        }).headline.account_id, "claude:0a1b2c3d4e5f");
    }

    function test_merge_patch_semantics() {
        const raw = Settings.decode(settingsJson);
        raw.display.future_option = 7;
        const merged = Settings.mergePatch(raw, Settings.displayPatch({
            valueMode: "left"
        }));
        compare(merged.display.value_mode, "left");
        compare(merged.display.future_option, 7);
        compare(merged.display.theme, "dark");
        compare(raw.display.value_mode, "used");
        compare(Settings.mergePatch(raw, Settings.headlinePatch({
            mode: "auto"
        })).headline, {
            mode: "auto"
        });
        compare(Settings.mergePatch(raw, {
            display: null
        }).display, undefined);
    }

    function test_patch_queue_keeps_order() {
        let step = PatchQueue.enqueue(PatchQueue.idle(), "a");
        compare(step.send, "a");
        step = PatchQueue.enqueue(step.queue, "b");
        compare(step.send, null);
        step = PatchQueue.enqueue(step.queue, "c");
        compare(step.queue.pending, ["b", "c"]);
        step = PatchQueue.settle(step.queue);
        compare(step.send, "b");
        step = PatchQueue.settle(step.queue);
        compare(step.send, "c");
        verify(!step.drained);
        step = PatchQueue.settle(step.queue);
        compare(step.send, null);
        verify(step.drained);
        compare(step.queue, PatchQueue.idle());
    }

    function test_toggles_and_hidden_windows() {
        const display = parse(settingsJson).display;
        compare(Settings.toggledValueMode(display), {
            valueMode: "left"
        });
        compare(Settings.toggledResetFormat(display), {
            resetFormat: "countdown"
        });
        verify(Settings.isWindowHidden(display, "codex:1a2b3c4d5e6f", "weekly"));
        compare(Settings.windowHiddenPatch(display, "codex:1a2b3c4d5e6f", "weekly", false), {
            hiddenWindows: {
                "codex:1a2b3c4d5e6f": null
            }
        });
        compare(Settings.windowHiddenPatch(display, "claude:0a1b2c3d4e5f", "model:opus", true), {
            hiddenWindows: {
                "claude:0a1b2c3d4e5f": ["model:opus"]
            }
        });
        compare(Settings.displayPatch(Settings.windowHiddenPatch(display, "codex:1a2b3c4d5e6f", "session", true)), {
            display: {
                hidden_windows: {
                    "codex:1a2b3c4d5e6f": ["weekly", "session"]
                }
            }
        });
        const cleared = Settings.mergePatch(Settings.decode(settingsJson), Settings.displayPatch(Settings.windowHiddenPatch(display, "codex:1a2b3c4d5e6f", "weekly", false)));
        compare(cleared.display.hidden_windows, {});
    }

    function test_options() {
        compare(Options.refreshOptions("en", 300).map(option => option.label).slice(0, 3), ["Every minute", "Every 2 minutes", "Every 5 minutes"]);
        compare(Options.refreshOptions("ru", 300).map(option => option.label).slice(0, 3), ["Каждую 1 минуту", "Каждые 2 минуты", "Каждые 5 минут"]);
        compare(Options.refreshOptions("ru", 90).map(option => option.label)[1], "90 секунд");
        const headline = {
            mode: "pinned",
            accountId: "codex:gone",
            window: "weekly"
        };
        const options = Options.limitOptions("en", null, headline);
        compare(options.map(option => option.label), ["Auto — most critical", "Pinned limit (not available now)"]);
        compare(Options.headlineFor(Options.headlineKey(headline)), headline);
        compare(Options.headlineFor("auto"), {
            mode: "auto"
        });
        compare(Options.indexOfValue(options, "missing"), 0);
        compare(Options.themeOptions("ru").map(option => option.label), ["Как в системе", "Светлая", "Тёмная"]);
    }

    function test_order() {
        compare(Order.moveItem(["a", "b", "c"], 0, 2), ["b", "c", "a"]);
        compare(Order.mergeOrder(["a", "h", "b", "c"], ["c", "a", "b"]), ["c", "h", "a", "b"]);
        compare(Order.reordered(["a", "h", "b"], ["a", "b"], 0, 1), ["b", "h", "a"]);
        compare(Order.targetIndex([10, 50, 90], 0, 70), 1);
        compare(Order.targetIndex([10, 50, 90], 2, 5), 0);
        compare(Order.indicatorSlot(0, 2, 3), {
            index: 2,
            below: true
        });
        compare(Order.indicatorSlot(0, 1, 3), {
            index: 2,
            below: false
        });
        compare(Order.indicatorSlot(2, 0, 3), {
            index: 0,
            below: false
        });
        compare(Order.indicatorSlot(1, 1, 3), null);
        compare(Order.indicatorSlot(-1, 0, 3), null);
    }

    function test_commands() {
        compare(Commands.shellQuote("it's"), "'it'\\''s'");
        const command = Commands.addAccountCommand("codex", " Ada's work ", "Press Enter");
        verify(command.startsWith("if command -v xdg-terminal-exec >/dev/null 2>&1; then exec xdg-terminal-exec sh -c '"));
        verify(command.includes("else exec konsole -e sh -c '"));
        verify(command.includes("headroom accounts add codex --label"));
        verify(!Commands.addAccountCommand("claude", "  ", "x").includes("--label"));
        compare(Commands.removeAccountCommand("codex:9f8e7d6c5b4a"), "headroom accounts remove 'codex:9f8e7d6c5b4a' --yes --progress json");
        for (const run of [() => Commands.addAccountCommand("rm -rf", "", ""), () => Commands.removeAccountCommand("x; rm -rf ~")]) {
            try {
                run();
                fail("expected a command error");
            } catch (error) {
                verify(Commands.isCommandError(error));
            }
        }
    }

    function test_command_script_round_trip() {
        const script = Commands.loginScript("codex", "Ada's", "Press Enter");
        compare(script, "headroom accounts add codex --label 'Ada'\\''s'; status=$?; printf '\\n%s ' 'Press Enter'; read -r _; exit $status");
    }

    function test_progress_outcome() {
        compare(Commands.progressOutcome("{\"event\":\"done\",\"account_id\":\"codex:1\",\"label\":null}\n", 0), {
            ok: true,
            message: ""
        });
        compare(Commands.progressOutcome("noise\n{\"event\":\"error\",\"message\":\"no such account\"}\n", 1), {
            ok: false,
            message: "no such account"
        });
        compare(Commands.progressOutcome("", 2).ok, false);
    }

    function test_motion() {
        compare(Motion.stagger(0, 0, 3), 0);
        compare(Motion.stagger(1, 2, 3), 1);
        verify(Motion.stagger(0.3, 0, 3) > Motion.stagger(0.3, 2, 3));
        compare(Motion.stagger(0.5, 0, 1), 0.5);
        compare(Motion.easeOutCubic(1), 1);
        compare(Motion.easeOutCubic(0), 0);
        verify(Motion.enabled({
            longDuration: 200
        }));
        verify(!Motion.enabled({
            longDuration: 0
        }));
        verify(!Motion.enabled({
            longDuration: 200
        }, true));
    }

    function test_popup_palette() {
        const system = {
            backgroundColor: Qt.rgba(0.94, 0.94, 0.95, 1),
            textColor: Qt.rgba(0.14, 0.15, 0.16, 1)
        };
        compare(Tokens.popupPalette(system, "system", false), null);
        const dark = Tokens.popupPalette(system, "dark", false);
        verify(dark.painted);
        compare(dark.textColor, "#dedede");
        compare(dark.backgroundColor.a, 1);
        const glass = Tokens.popupPalette(system, "system", true);
        verify(!glass.painted);
        verify(glass.backgroundColor.a < 1);
    }
}
