import QtQuick
import QtTest
import org.kde.plasma.plasmoid
import org.kde.plasma.workspace.dbus as DBus
import headroom.preview

TestCase {
    id: suite

    readonly property int settleMs: 3000
    readonly property int replyDrainMs: 100
    property Item plasmoid: null

    function findAll(object, predicate, found) {
        if (predicate(object))
            found.push(object);
        for (const child of object.data ?? [])
            findAll(child, predicate, found);
        return found;
    }

    function named(name) {
        return findAll(plasmoid, item => item.objectName === name, [])[0] ?? null;
    }

    function sections() {
        return findAll(plasmoid, item => item.account !== undefined && item.notices !== undefined && item.windows !== undefined, []);
    }

    function refreshCalls() {
        return DBus.SessionBus.messages.filter(message => message.member === "Refresh");
    }

    function open(scenario, language) {
        PreviewConfig.scenario = scenario;
        PreviewConfig.displayPatch = {
            language: language ?? "en"
        };
        PreviewConfig.appliedSettings = null;
        plasmoid = createTemporaryObject(plasmoidComponent, suite) as Item;
        tryVerify(() => sections().length > 0, settleMs);
        DBus.SessionBus.messages = [];
        Plasmoid.triggeredActions = [];
    }

    function cleanup() {
        wait(replyDrainMs);
        PreviewConfig.scenario = "ready";
        PreviewConfig.displayPatch = {};
        plasmoid = null;
    }

    function test_refresh_button_calls_refresh_for_all_accounts() {
        open("ready");
        const button = named("refreshButton");
        verify(button !== null);
        compare(button.Accessible.name, "Refresh");
        verify(!button.spinning);
        mouseClick(button);
        tryVerify(() => refreshCalls().length === 1, settleMs);
        compare(refreshCalls()[0].arguments, [""]);
        compare(refreshCalls()[0].signature, "(s)");
        compare(Plasmoid.triggeredActions, []);
    }

    function test_settings_button_opens_configuration() {
        open("ready");
        const button = named("settingsButton");
        verify(button !== null);
        compare(button.Accessible.name, "Settings");
        mouseClick(button);
        compare(Plasmoid.triggeredActions, ["configure"]);
        compare(refreshCalls(), []);
    }

    function test_refresh_button_spins_while_refreshing() {
        open("refreshing");
        verify(named("refreshButton").spinning);
    }

    function test_popup_has_no_options_menu() {
        open("ready");
        compare(findAll(plasmoid, item => String(item).startsWith("Options"), []), []);
    }

    function test_no_subscription_card_has_notice_without_meters() {
        open("ready");
        const section = sections().find(item => item.account.status === "no_subscription");
        verify(section !== undefined);
        compare(section.windows, []);
        compare(section.notices.map(notice => notice.title), ["No active subscription"]);
        compare(section.notices[0].note, "the account has no active Codex plan");
    }

    function test_russian_button_names() {
        open("ready", "ru");
        compare(named("refreshButton").Accessible.name, "Обновить");
        compare(named("settingsButton").Accessible.name, "Настройки");
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
}
