import QtQuick
import QtTest
import headroom.preview
import "../package/contents/ui/logic/Settings.js" as Settings
import "../package/contents/ui/logic/State.js" as State
import "../package/contents/ui/logic/Update.js" as Update

TestCase {
    id: suite

    readonly property int settleMs: 3000
    readonly property int replyDrainMs: 100
    readonly property string releaseUrl: "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0"
    property Item root: null

    function raw(install, extra) {
        return Object.assign({
            version: "0.5.0",
            url: releaseUrl,
            published_at: "2026-09-20T12:00:00Z",
            install,
            command: "yay -Syu headroom"
        }, extra ?? {});
    }

    function findAll(object, predicate, found) {
        if (predicate(object))
            found.push(object);
        for (const child of object.data ?? [])
            findAll(child, predicate, found);
        return found;
    }

    function named(name) {
        return findAll(root, item => item.objectName === name, [])[0] ?? null;
    }

    function load(component, scenario) {
        PreviewConfig.scenario = scenario;
        PreviewConfig.displayPatch = {
            language: "en"
        };
        PreviewConfig.appliedSettings = null;
        root = createTemporaryObject(component, suite) as Item;
    }

    function openPopup(scenario) {
        load(plasmoidComponent, scenario);
        tryVerify(() => named("updateRow") !== null && named("updateTitle")?.text !== "", settleMs);
    }

    function cleanup() {
        wait(replyDrainMs);
        PreviewConfig.scenario = "ready";
        PreviewConfig.displayPatch = {};
        PreviewConfig.appliedSettings = null;
        root = null;
    }

    function test_parse_update() {
        compare(Update.parseUpdate(null), null);
        compare(Update.parseUpdate({
            url: releaseUrl
        }), null);
        const update = Update.parseUpdate(raw("self"));
        compare(update.version, "0.5.0");
        compare(update.publishedAt.toISOString(), "2026-09-20T12:00:00.000Z");
        compare(Update.parseUpdate(raw("snap")).install, "unknown");
        compare(Update.parseUpdate(raw("self", {
            url: "javascript:alert(1)"
        })).url, null);
        compare(State.parseState("{\"version\": 1}").update, null);
        compare(State.parseState(JSON.stringify({
            version: 1,
            update: raw("package")
        })).update.install, "package");
    }

    function test_action_per_install_kind() {
        compare(["self", "package", "unknown"].map(kind => Update.action(Update.parseUpdate(raw(kind)))), ["install", "command", "notes"]);
        compare(Update.action(Update.parseUpdate(raw("package", {
            command: " "
        }))), "notes");
        compare(Update.action(Update.parseUpdate(raw("unknown", {
            url: null
        }))), "");
        compare(["self", "package", "unknown"].map(kind => Update.showsWhatsNew(Update.parseUpdate(raw(kind)))), [true, true, false]);
        compare(Update.actionLabel("ru", "install"), "Обновить");
    }

    function test_outcome_and_lines() {
        compare(Update.runLine("en", Update.IDLE), "");
        compare(Update.runLine("en", Update.started()), "Updating Headroom…");
        const done = Update.outcome("{\"event\":\"step\",\"text\":\"Downloading\"}\n{\"event\":\"done\",\"version\":\"0.5.0\",\"relogin\":true}\n", 0);
        compare(Update.runLine("en", done), "Updated — log out and back in to finish");
        compare(Update.runLine("ru", done), "Обновлено — выйдите из сеанса и войдите снова");
        compare(Update.runLine("en", Update.outcome("{\"event\":\"done\",\"version\":\"0.5.0\"}", 0)), "Updated to 0.5.0");
        const failed = Update.outcome("{\"event\":\"error\",\"message\":\"checksum mismatch\"}\n", 1);
        compare([failed.phase, Update.runLine("en", failed)], ["failed", "checksum mismatch"]);
        compare(Update.runLine("en", Update.outcome("", 127)), "Couldn't find the headroom command");
        compare(Update.runLine("en", Update.outcome("{\"event\":\"done\"}", 3)), "The update stopped before it finished");
        compare(Update.title("ru", Update.parseUpdate(raw("self"))), "Доступна версия Headroom 0.5.0");
    }

    function test_updates_setting() {
        compare(Settings.fromRaw({}).updates, {
            check: true
        });
        compare(Settings.fromRaw({
            updates: {
                check: false
            }
        }).updates.check, false);
        compare(Settings.updatesPatch(false), {
            updates: {
                check: false
            }
        });
    }

    function test_popup_hides_row_without_update() {
        load(plasmoidComponent, "ready");
        tryVerify(() => named("updateRow") !== null, settleMs);
        wait(200);
        verify(!named("updateRow").visible);
    }

    function test_popup_self_update_runs_command() {
        openPopup("update-self");
        verify(named("updateRow").visible);
        compare(named("updateTitle").text, "Headroom 0.5.0 is available");
        verify(named("updateWhatsNew").visible);
        const action = named("updateAction");
        compare(action.text, "Update");
        verify(action.primary);
        mouseClick(action);
        const runner = named("updateRunner");
        compare(runner.connectedSources, [Update.COMMAND]);
        verify(named("updateSpinner").visible);
        verify(!action.visible);
        runner.newData(Update.COMMAND, {
            "exit code": 0,
            stdout: "{\"event\":\"done\",\"version\":\"0.5.0\",\"relogin\":true}\n"
        });
        tryCompare(named("updateLine"), "text", "Updated — log out and back in to finish");
        verify(!action.visible);
    }

    function test_popup_failed_update_offers_retry() {
        openPopup("update-self");
        mouseClick(named("updateAction"));
        named("updateRunner").newData(Update.COMMAND, {
            "exit code": 1,
            stdout: "{\"event\":\"error\",\"message\":\"checksum mismatch\"}\n"
        });
        tryCompare(named("updateAction"), "text", "Retry");
        compare(named("updateLine").text, "checksum mismatch");
        verify(!named("updateAction").primary);
    }

    function test_popup_package_shows_command() {
        openPopup("update-package");
        const action = named("updateAction");
        compare(action.text, "How to update");
        verify(!named("updateCommand").visible);
        mouseClick(action);
        verify(named("updateCommand").visible);
        compare(named("updateCopy").text, "Copy");
        mouseClick(named("updateCopy"));
        compare(named("updateCopy").text, "Copied");
        compare(named("updateRunner").connectedSources, []);
    }

    function test_popup_unknown_offers_release_notes() {
        openPopup("update-unknown");
        compare(named("updateAction").text, "Release notes");
        verify(!named("updateWhatsNew").visible);
    }

    function test_config_toggle_and_release_row() {
        load(configComponent, "update-package");
        tryVerify(() => named("updateRelease")?.visible === true, settleMs);
        compare(named("updateRelease").title, "Headroom 0.5.0 is available");
        compare(named("updateReleaseAction").text, "Copy");
        const check = named("updatesCheck");
        verify(check.checked);
        mouseClick(check);
        tryVerify(() => PreviewConfig.appliedSettings?.updates?.check === false, settleMs);
    }

    width: 800
    height: 1600
    visible: true
    when: windowShown

    Component {
        id: plasmoidComponent

        Loader {
            source: "../package/contents/ui/main.qml"
        }
    }

    Component {
        id: configComponent

        Loader {
            width: 700
            height: 1600
            source: "../package/contents/ui/ConfigGeneral.qml"
        }
    }
}
