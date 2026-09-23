import QtQuick
import QtTest
import "../package/contents/ui/logic/Commands.js" as Commands
import "../package/contents/ui/logic/Providers.js" as Providers
import "../package/contents/ui/logic/Registry.js" as Registry

TestCase {
    function read(relative) {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl(relative), false);
        request.send();
        return request.responseText;
    }

    function sample() {
        return Registry.parseRegistry(read("../dev/sample-providers.json"));
    }

    function provider(id) {
        return Registry.findProvider(sample(), id);
    }

    function registryJson(providers) {
        return JSON.stringify({
            version: 1,
            providers
        });
    }

    function test_parses_sample_registry() {
        const providers = sample();
        compare(providers.length, 14);
        compare(providers.slice(0, 4).map(entry => entry.id), ["codex", "claude", "opencode", "openrouter"]);
        compare(provider("codex"), {
            id: "codex",
            name: "Codex",
            method: {
                kind: "cli_login",
                program: "codex",
                label: null,
                consoleUrl: null,
                hint: null,
                reason: null
            },
            multiAccount: true,
            localUsage: true
        });
        compare(provider("openrouter").method.kind, "api_key");
        compare(provider("openrouter").method.consoleUrl, "https://openrouter.ai/settings/keys");
        compare(provider("cursor").method.kind, "auto_detect");
        compare(Registry.findProvider(providers, "missing"), null);
    }

    function test_skips_unusable_entries() {
        const providers = Registry.parseRegistry(registryJson([
            {
                id: "rm -rf ~",
                display_name: "Bad",
                add_account: [
                    {
                        kind: "cli_login",
                        program: "x"
                    }
                ]
            },
            {
                id: "future",
                display_name: "Future",
                add_account: [
                    {
                        kind: "passkey"
                    },
                    {
                        kind: "cli_login",
                        program: "future"
                    }
                ]
            },
            {
                id: "empty",
                add_account: []
            },
            {
                id: "keyed",
                add_account: [
                    {
                        kind: "api_key",
                        console_url: "javascript:alert(1)",
                        extra: true
                    }
                ]
            },
            "noise"]));
        compare(providers.map(entry => entry.id), ["keyed"]);
        compare(providers[0].name, "keyed");
        compare(providers[0].method.consoleUrl, null);
        compare(providers[0].multiAccount, false);
    }

    function test_registry_errors() {
        for (const json of ["{", "[]", "{\"version\": 1}", "{\"version\": 2, \"providers\": []}"]) {
            try {
                Registry.parseRegistry(json);
                fail(`expected a registry error for ${json}`);
            } catch (error) {
                verify(Registry.isRegistryError(error), error.message);
            }
        }
    }

    function test_method_hints() {
        compare(Registry.methodHint("en", provider("claude")), "Signs in with the claude CLI in a terminal");
        compare(Registry.methodHint("ru", provider("claude")), "Вход через CLI claude в терминале");
        compare(Registry.methodHint("en", provider("zai")), "Asks for an API key in a terminal");
        compare(Registry.methodHint("en", provider("opencode")), "Found from OpenCode's local logs; nothing to sign in to.");
        compare(Registry.keyHint(provider("openrouter").method), "OpenRouter API key: Starts with sk-or-");
        compare(Registry.keyHint(provider("codex").method), "");
    }

    function test_cli_login_plan_opens_terminal() {
        const plan = Commands.addPlan(provider("codex"), "Work", "Press Enter");
        compare(plan.kind, "terminal");
        verify(plan.command.includes("headroom accounts add codex --label"));
        verify(plan.command.startsWith("if command -v xdg-terminal-exec"));
    }

    function test_api_key_plan_never_carries_a_key() {
        const plan = Commands.addPlan(provider("openrouter"), "", "Press Enter");
        compare(plan.kind, "terminal");
        compare(plan.command, Commands.addAccountCommand("openrouter", "", "Press Enter"));
        verify(plan.command.includes("headroom accounts add openrouter;"));
        verify(!plan.command.includes("--api-key"));
        verify(!plan.command.includes("sk-"));
        verify(!plan.command.includes("--label"));
    }

    function test_auto_detect_plan_rescans() {
        compare(Commands.addPlan(provider("cursor"), "ignored", "Press Enter"), {
            kind: "rescan",
            command: ""
        });
    }

    function test_plan_rejects_unsafe_ids() {
        try {
            Commands.addPlan({
                id: "x; rm -rf ~",
                method: {
                    kind: "cli_login"
                }
            }, "", "");
            fail("expected a command error");
        } catch (error) {
            verify(Commands.isCommandError(error));
        }
    }

    function test_icons_exist_with_generic_fallback() {
        for (const id of Providers.ICONS) {
            compare(Providers.iconFile(id), `${id}.svg`);
            verify(read(`../package/contents/icons/${id}.svg`).includes("<svg"), id);
        }
        compare(Providers.iconFile("devin"), "provider-generic.svg");
        verify(read("../package/contents/icons/provider-generic.svg").includes("<svg"));
        verify(!Providers.isTinted("claude"));
        verify(Providers.isTinted("openrouter"));
        verify(Providers.isTinted("unknown"));
    }

    function test_ring_colors_are_stable_and_distinct() {
        const ids = sample().map(entry => entry.id);
        for (const dark of [false, true]) {
            const colors = ids.map(id => Providers.ringColor(id, dark));
            compare(new Set(colors).size, ids.length);
        }
        compare(Providers.ringColor("codex", false), "#10A37F");
        compare(Providers.ringColor("claude", false), "#D97757");
        compare(Providers.ringColor("claude", true), "#D97757");
        compare(Providers.ringColor("newcomer", false), Providers.ringColor("newcomer", false));
        verify(Providers.ringColor("newcomer", true) !== Providers.ringColor("newcomer", false));
    }
}
