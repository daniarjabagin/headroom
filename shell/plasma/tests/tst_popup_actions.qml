import QtQuick
import QtTest
import org.kde.plasma.plasmoid
import org.kde.plasma.workspace.dbus as DBus
import headroom.preview
import "../package/contents/ui/logic/Commands.js" as Commands
import "../package/contents/ui/logic/FormatSpend.js" as FormatSpend

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

    function test_signed_out_account_signs_in_through_terminal() {
        open("ready");
        const section = sections().find(item => item.account.status === "signed_out");
        verify(section !== undefined);
        tryVerify(() => JSON.stringify(section.notices[0].actions.map(action => action.kind)) === '["signin","retry"]', settleMs);
        section.runAction("signin", section.account.provider);
        const runner = findAll(plasmoid, item => item.engine === "executable", [])[0];
        verify(runner !== undefined);
        const command = Commands.addAccountCommand(section.account.provider, "", "Press Enter to close this window");
        compare(runner.connectedSources, [command]);
        runner.newData(command, {
            "exit code": 0,
            stdout: ""
        });
        tryVerify(() => calls("Rescan").length === 1, settleMs);
        compare(refreshCalls(), []);
    }

    function test_signed_out_retry_shows_progress_while_refreshing() {
        open("retrying");
        const section = sections().find(item => item.account.id === "claude:5e4d3c2b1a0f");
        verify(section !== undefined);
        compare(section.account.status, "refreshing");
        compare(section.notices.map(notice => notice.title), ["Signed out of Claude"]);
        const retry = findAll(section, item => item.busy !== undefined && item.text === "Retry", [])[0];
        verify(retry !== undefined);
        verify(retry.busy);
        verify(!retry.enabled);
        const spinner = findAll(retry, item => item.objectName === "smallButtonSpinner", [])[0];
        verify(spinner.visible);
        verify(spinner.spinning);
    }

    function test_signed_out_retry_is_idle_when_not_refreshing() {
        open("ready");
        const section = sections().find(item => item.account.status === "signed_out");
        const retry = findAll(section, item => item.busy !== undefined && item.text === "Retry", [])[0];
        verify(!retry.busy);
        verify(retry.enabled);
        retry.clicked();
        tryVerify(() => refreshCalls().length === 1, settleMs);
        compare(refreshCalls()[0].arguments, [section.account.id]);
    }

    function test_sign_in_without_terminal_opens_configuration() {
        open("ready");
        const section = sections().find(item => item.account.status === "signed_out");
        section.runAction("settings", section.account.provider);
        compare(Plasmoid.triggeredActions, ["configure"]);
        compare(refreshCalls(), []);
    }

    function test_single_provider_spend_keeps_the_donut() {
        open("single-spend");
        const card = findAll(plasmoid, item => item.body !== undefined && item.periodSelected !== undefined, [])[0];
        verify(card !== undefined);
        compare(card.current.providers.length, 1);
        compare(card.body, "ring");
        const donut = findAll(card, item => item.slices !== undefined && item.holeRatio !== undefined, [])[0];
        verify(donut.visible);
        compare(donut.slices.length, 1);
        compare(donut.slices[0].sweep, 360);
        const tokens = findAll(card, item => item.objectName === "legendTokens", [])[0];
        verify(tokens.visible);
        compare(tokens.text, FormatSpend.tokenCount("en", card.current.providers[0].totalTokens));
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
