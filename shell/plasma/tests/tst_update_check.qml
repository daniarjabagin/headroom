import QtQuick
import QtTest
import org.kde.plasma.workspace.dbus as DBus
import headroom.preview
import "../package/contents/ui/logic/State.js" as State
import "../package/contents/ui/logic/UpdateCheck.js" as UpdateCheck

TestCase {
    id: suite

    readonly property int settleMs: 3000
    readonly property int replyDrainMs: 100
    readonly property date now: new Date("2026-09-23T10:00:00Z")
    readonly property string noReply: "org.freedesktop.DBus.Error.NoReply"
    property Item root: null

    function check(status, checkedAt, extra) {
        return Object.assign({
            status,
            checkedAt: checkedAt === null ? null : new Date(checkedAt),
            version: null,
            until: null
        }, extra ?? {});
    }

    function snapshot(extra) {
        return Object.assign({
            update: null,
            updateCheck: {
                checkedAt: new Date("2026-09-23T09:55:00Z")
            },
            appVersion: "0.6.0"
        }, extra ?? {});
    }

    function test_parse_update_check() {
        compare(UpdateCheck.parseUpdateCheck(null), null);
        compare(UpdateCheck.parseUpdateCheck({
            checked_at: "2026-09-23T04:00:00Z"
        }).checkedAt.toISOString(), "2026-09-23T04:00:00.000Z");
        compare(UpdateCheck.parseUpdateCheck({}).checkedAt, null);
        const state = State.parseState(JSON.stringify({
            version: 1,
            app_version: "0.6.0",
            update_check: {
                checked_at: null
            }
        }));
        compare(state.appVersion, "0.6.0");
        compare(state.updateCheck, {
            checkedAt: null
        });
        const old = State.parseState("{\"version\": 1}");
        compare(old.appVersion, null);
        compare(old.updateCheck, null);
    }

    function test_supported_versions() {
        compare(["0.6.0", "0.6.1", "0.7.0", "1.0.0", "v0.6.0", "0.6.0-dev", "0.5.9", "0.5.1", "", null, "six"].map(UpdateCheck.supported), [true, true, true, true, true, true, false, false, false, false, false]);
        verify(UpdateCheck.visible(true, snapshot()));
        verify(!UpdateCheck.visible(false, snapshot()));
        verify(!UpdateCheck.visible(true, null));
        verify(!UpdateCheck.visible(true, snapshot({
            updateCheck: null
        })));
        verify(!UpdateCheck.visible(true, snapshot({
            appVersion: "0.5.1"
        })));
    }

    function test_outcome() {
        const previous = new Date("2026-09-22T09:14:00Z");
        compare(UpdateCheck.outcome("", "{\"status\":\"rate_limited\",\"checked_at\":\"2026-09-22T09:14:00Z\",\"version\":\"0.5.1\",\"until\":\"2026-09-23T11:30:00Z\"}", null), check("rate_limited", "2026-09-22T09:14:00Z", {
            version: "0.5.1",
            until: new Date("2026-09-23T11:30:00Z")
        }));
        compare(UpdateCheck.outcome("", "{\"status\":\"available\",\"checked_at\":\"2026-09-23T10:00:00Z\",\"version\":\"0.6.1\"}", null).status, "available");
        compare(UpdateCheck.outcome("", "not json", previous), check("failed", "2026-09-22T09:14:00Z"));
        compare(UpdateCheck.outcome("", "{\"status\":\"exploded\"}", previous).status, "failed");
        compare(UpdateCheck.outcome("org.freedesktop.DBus.Error.NotSupported", null, previous).status, "disabled");
        compare(UpdateCheck.outcome("org.freedesktop.DBus.Error.Failed", null, previous), check("failed", "2026-09-22T09:14:00Z"));
    }

    function test_retries_only_timeouts() {
        verify(UpdateCheck.retries(noReply, 1));
        verify(UpdateCheck.retries("org.freedesktop.DBus.Error.Timeout", 2));
        verify(!UpdateCheck.retries(noReply, 3));
        verify(!UpdateCheck.retries("", 1));
        verify(!UpdateCheck.retries("org.freedesktop.DBus.Error.Failed", 1));
    }

    function test_status_wording() {
        const line = (lang, value) => UpdateCheck.statusLine(lang, value, "0.6.0", now);
        compare(line("en", check("up_to_date", "2026-09-23T09:59:30Z")), "You're up to date · Headroom 0.6.0 · checked just now");
        compare(line("ru", check("up_to_date", "2026-09-23T09:55:00Z")), "У вас последняя версия · Headroom 0.6.0 · проверено 5 мин назад");
        compare(line("en", check("up_to_date", null)), "You're up to date · Headroom 0.6.0");
        compare(line("en", check("available", "2026-09-23T08:00:00Z", {
            version: "0.6.1"
        })), "Update available · checked 2h 0m ago");
        compare(line("ru", check("available", null)), "Доступно обновление");
        compare(line("en", check("failed", "2026-09-23T09:50:00Z")), "Couldn't check for updates · last checked 10m ago");
        compare(line("en", check("failed", null)), "Couldn't check for updates");
        compare(line("ru", check("failed", null)), "Не удалось проверить обновления");
        compare(line("en", check("rate_limited", null, {
            until: new Date("2026-09-23T11:30:00Z")
        })), "GitHub limits update checks · try again in 1h 30m");
        compare(line("en", check("rate_limited", null)), "GitHub limits update checks · try again later");
        compare(line("en", check("disabled", null)), "Update checks are off");
        compare(line("en", check("unchecked", null)), "Headroom 0.6.0 · not checked yet");
        compare(line("ru", check("unchecked", null)), "Headroom 0.6.0 · ещё не проверялось");
    }

    function test_shown_prefers_state_after_success() {
        compare(UpdateCheck.shown(snapshot(), null).status, "up_to_date");
        compare(UpdateCheck.shown(snapshot({
            update: {
                version: "0.6.1"
            }
        }), null).status, "available");
        compare(UpdateCheck.shown(snapshot({
            updateCheck: {
                checkedAt: null
            }
        }), null).status, "unchecked");
        const failed = check("failed", null);
        compare(UpdateCheck.shown(snapshot(), failed), failed);
        compare(UpdateCheck.shown(snapshot(), check("up_to_date", "2026-09-20T00:00:00Z")).checkedAt.toISOString(), "2026-09-23T09:55:00.000Z");
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

    function checkCalls() {
        return DBus.SessionBus.messages.filter(message => message.member === "CheckForUpdates");
    }

    function loadConfig(appVersion) {
        PreviewConfig.scenario = "ready";
        PreviewConfig.displayPatch = {
            language: "en"
        };
        PreviewConfig.appliedSettings = null;
        PreviewConfig.statePatch = {
            app_version: appVersion
        };
        root = createTemporaryObject(configComponent, suite) as Item;
        tryVerify(() => named("updateCheckRow") !== null && named("updatesCheck") !== null, settleMs);
        wait(replyDrainMs);
        DBus.SessionBus.messages = [];
    }

    function cleanup() {
        wait(replyDrainMs);
        PreviewConfig.scenario = "ready";
        PreviewConfig.displayPatch = {};
        PreviewConfig.appliedSettings = null;
        PreviewConfig.statePatch = {};
        PreviewConfig.checkReplies = [];
        root = null;
    }

    function test_config_check_now_retries_past_the_bus_timeout() {
        loadConfig("0.6.0");
        const row = named("updateCheckRow");
        tryVerify(() => row.visible, settleMs);
        verify(row.title.startsWith("You're up to date · Headroom 0.6.0 · checked "));
        PreviewConfig.checkReplies = [
            {
                isError: true,
                error: {
                    name: noReply,
                    message: "Did not receive a reply"
                }
            },
            {
                value: JSON.stringify({
                    status: "rate_limited",
                    checked_at: null,
                    version: null
                })
            }
        ];
        const button = named("updateCheckNow");
        compare(button.text, "Check now");
        mouseClick(button);
        verify(named("updateCheckBusy").visible);
        verify(!button.enabled);
        tryVerify(() => !named("updateCheckBusy").visible, settleMs);
        compare(checkCalls().length, 2);
        compare(checkCalls()[0].arguments, []);
        compare(row.title, "GitHub limits update checks · try again later");
        verify(button.enabled);
    }

    function test_config_check_row_hidden_for_old_daemon_or_checks_off() {
        loadConfig("0.5.1");
        wait(replyDrainMs);
        verify(!named("updateCheckRow").visible);
        cleanup();
        loadConfig("0.6.0");
        tryVerify(() => named("updateCheckRow").visible, settleMs);
        mouseClick(named("updatesCheck"));
        tryVerify(() => !named("updateCheckRow").visible, settleMs);
    }

    width: 800
    height: 1600
    visible: true
    when: windowShown

    Component {
        id: configComponent

        Loader {
            width: 700
            height: 1600
            source: "../package/contents/ui/ConfigGeneral.qml"
        }
    }
}
