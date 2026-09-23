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

    function calls(member) {
        return DBus.SessionBus.messages.filter(message => message.member === member);
    }

    function refreshCalls() {
        return calls("Refresh");
    }

    function refreshNowCalls() {
        return calls("RefreshNow");
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
        PreviewConfig.failingMembers = [];
        plasmoid = null;
    }

    function test_refresh_button_calls_refresh_now() {
        open("ready");
        const button = named("refreshButton");
        verify(button !== null);
        compare(button.Accessible.name, "Refresh");
        verify(!button.spinning);
        mouseClick(button);
        verify(button.spinning);
        tryVerify(() => refreshNowCalls().length === 1, settleMs);
        compare(refreshNowCalls()[0].arguments, []);
        compare(refreshNowCalls()[0].signature, "()");
        compare(refreshCalls(), []);
        compare(Plasmoid.triggeredActions, []);
    }

    function test_refresh_button_spins_at_least_hold_time() {
        open("ready");
        const button = named("refreshButton");
        mouseClick(button);
        wait(button.holdMs / 2);
        verify(button.spinning);
        tryVerify(() => !button.spinning, settleMs);
        verify(!button.failed);
    }

    function test_refresh_button_ignores_repeat_clicks_during_hold() {
        open("ready");
        const button = named("refreshButton");
        mouseClick(button);
        mouseClick(button);
        wait(replyDrainMs);
        compare(refreshNowCalls().length, 1);
    }

    function test_refresh_button_keeps_spinning_while_refreshing() {
        open("refreshing");
        const button = named("refreshButton");
        mouseClick(button);
        wait(button.holdMs + replyDrainMs);
        verify(button.spinning);
        compare(refreshNowCalls().length, 1);
    }

    function test_refresh_failure_stops_spin_and_tints() {
        open("ready");
        PreviewConfig.failingMembers = ["RefreshNow"];
        const button = named("refreshButton");
        mouseClick(button);
        tryVerify(() => button.failed, settleMs);
        verify(!button.spinning);
        tryVerify(() => !button.failed, settleMs);
    }

    function test_refresh_glyph_settles_on_full_turn() {
        open("ready");
        const button = named("refreshButton");
        const glyph = named("refreshGlyph");
        mouseClick(button);
        tryVerify(() => glyph.rotation > 0, settleMs);
        tryVerify(() => !glyph.turning, settleMs * 2);
        compare(glyph.rotation, 0);
    }

    function test_footer_status_click_refreshes_now_with_header_spin() {
        open("ready");
        const status = named("footerStatus");
        verify(status !== null);
        verify(status.enabled && status.visible);
        const button = named("refreshButton");
        verify(!button.spinning);
        mouseClick(status);
        verify(button.spinning);
        tryVerify(() => refreshNowCalls().length === 1, settleMs);
        compare(refreshNowCalls()[0].signature, "()");
        compare(refreshCalls(), []);
    }

    function test_account_retry_refreshes_that_account() {
        open("ready");
        const section = sections().find(item => item.account.status === "error");
        verify(section !== undefined);
        section.refreshRequested(section.account.id);
        tryVerify(() => refreshCalls().length === 1, settleMs);
        compare(refreshCalls()[0].arguments, [section.account.id]);
        compare(refreshNowCalls(), []);
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

    function test_refresh_glyph_rests_without_animations() {
        open("refreshing");
        const button = named("refreshButton");
        const glyph = named("refreshGlyph");
        button.animated = false;
        verify(button.spinning);
        verify(!glyph.turning);
        compare(glyph.rotation, 0);
    }

    function test_refresh_button_spins_while_refreshing() {
        open("refreshing");
        verify(named("refreshButton").spinning);
        verify(named("refreshGlyph").turning);
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
