import QtQuick
import QtTest
import "../package/contents/ui/logic/Commands.js" as Commands
import "../package/contents/ui/logic/I18n.js" as I18n
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
        try {
            parse("[]");
            fail("expected a settings error");
        } catch (error) {
            compare(error.message, "Unexpected settings from the Headroom service");
            compare(I18n.errorText("ru", error), "Служба Headroom прислала настройки в неожиданном формате");
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

    function test_patch_queue_coalesces_waiting_patches() {
        let step = PatchQueue.enqueue(PatchQueue.idle(), Settings.displayPatch({
            theme: "dark"
        }));
        step = PatchQueue.enqueue(step.queue, Settings.displayPatch({
            valueMode: "used"
        }));
        step = PatchQueue.enqueue(step.queue, Settings.displayPatch({
            valueMode: "left"
        }));
        step = PatchQueue.enqueue(step.queue, {
            display: {
                hidden_windows: {
                    a: null
                }
            }
        });
        compare(step.queue.pending, [
            {
                display: {
                    value_mode: "left",
                    hidden_windows: {
                        a: null
                    }
                }
            }
        ]);
        step = PatchQueue.settle(step.queue);
        compare(step.send.display.value_mode, "left");
        verify(PatchQueue.settle(step.queue).drained);
    }

    function test_patch_queue_keeps_patches_that_do_not_compose() {
        let step = PatchQueue.enqueue(PatchQueue.idle(), {
            x: 1
        });
        step = PatchQueue.enqueue(step.queue, {
            display: null
        });
        step = PatchQueue.enqueue(step.queue, {
            display: {
                theme: "dark"
            }
        });
        compare(step.queue.pending, [
            {
                display: null
            },
            {
                display: {
                    theme: "dark"
                }
            }
        ]);
    }

    function test_coalesced_patch_applies_like_the_sequence() {
        const raw = Settings.decode(settingsJson);
        const patches = [Settings.displayPatch({
                theme: "dark"
            }), Settings.displayPatch({
                valueMode: "left"
            }),
            {
                display: {
                    hidden_windows: null
                }
            },
            Settings.displayPatch({
                theme: "light"
            })];
        const sequential = patches.reduce((current, patch) => Settings.mergePatch(current, patch), raw);
        let step = PatchQueue.enqueue(PatchQueue.idle(), {
            first: true
        });
        patches.forEach(patch => step = PatchQueue.enqueue(step.queue, patch));
        compare(step.queue.pending.length, 1);
        compare(Settings.mergePatch(raw, step.queue.pending[0]), sequential);
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
        verify(command.includes("headroom accounts add codex --label="));
        verify(!Commands.addAccountCommand("claude", "  ", "x").includes("--label"));
        compare(Commands.removeAccountCommand("codex:9f8e7d6c5b4a"), "headroom accounts remove 'codex:9f8e7d6c5b4a' --yes --progress json");
        const login = Commands.signInCommand("claude:5e4d3c2b1a0f", "Press Enter");
        verify(login.includes("xdg-terminal-exec sh -c "));
        verify(login.includes("headroom accounts login '\\''claude:5e4d3c2b1a0f'\\''; status=$?"));
        compare(Commands.signInCommand("codex", "Press Enter"), Commands.addAccountCommand("codex", "", "Press Enter"));
        for (const run of [() => Commands.addAccountCommand("rm -rf", "", ""), () => Commands.addAccountCommand("-h", "", ""), () => Commands.removeAccountCommand("x; rm -rf ~"), () => Commands.removeAccountCommand("-x:1a"), () => Commands.loginAccountCommand("claude:1a; rm", ""), () => Commands.signInCommand("-x:1a", "")]) {
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
        compare(script, "headroom accounts add codex --label='Ada'\\''s'; status=$?; printf '\\n%s ' 'Press Enter'; read -r _; exit $status");
    }

    function test_label_starting_with_dash_stays_a_value() {
        compare(Commands.loginScript("codex", " --yes ", "x"), "headroom accounts add codex --label='--yes'; status=$?; printf '\\n%s ' 'x'; read -r _; exit $status");
        verify(Commands.addAccountCommand("codex", "-work", "x").includes("--label='\\''-work'\\''"));
    }

    function test_command_errors_are_translated() {
        try {
            Commands.removeAccountCommand("bogus");
            fail("expected a command error");
        } catch (error) {
            compare(error.message, "Unexpected account id bogus");
            compare(I18n.errorText("ru", error), "Недопустимый идентификатор аккаунта bogus");
        }
    }

    function test_pending_command_kinds_match_exactly() {
        const spoof = Commands.addAccountCommand("codex", "accounts remove", "x");
        const remove = Commands.removeAccountCommand("codex:1a");
        const pending = Commands.track(Commands.track({}, spoof, "add"), remove, "remove");
        const first = Commands.settle(pending, spoof);
        compare(first.kind, "add");
        compare(Object.keys(first.pending), [remove]);
        const second = Commands.settle(first.pending, remove);
        compare(second.kind, "remove");
        compare(second.pending, {});
        compare(Commands.settle(second.pending, remove).kind, "");
        compare(Commands.settle({}, "toString").kind, "");
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
