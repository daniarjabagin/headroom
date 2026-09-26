import QtQuick
import QtTest
import headroom.preview
import "../package/contents/ui"
import "../package/contents/ui/logic/Commands.js" as Commands
import "../package/contents/ui/logic/SpendState.js" as SpendState

TestCase {
    id: suite

    readonly property int settleMs: 3000
    readonly property int replyDrainMs: 100
    readonly property string signedOutId: "claude:5e4d3c2b1a0f"
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

    function host() {
        return (plasmoid as Loader).item;
    }

    function allNamed(name) {
        return findAll(plasmoid, item => item.objectName === name, []);
    }

    function sections() {
        return findAll(plasmoid, item => item.account !== undefined && item.notices !== undefined && item.windows !== undefined, []);
    }

    function rawSample() {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("../dev/sample-state.json"), false);
        request.send();
        return JSON.parse(request.responseText);
    }

    function open(display, statePatch) {
        PreviewConfig.scenario = "ready";
        PreviewConfig.displayPatch = Object.assign({
            language: "en"
        }, display ?? {});
        PreviewConfig.statePatch = statePatch ?? {};
        PreviewConfig.appliedSettings = null;
        plasmoid = createTemporaryObject(plasmoidComponent, suite) as Item;
        tryVerify(() => sections().length > 0, settleMs);
    }

    function cleanup() {
        wait(replyDrainMs);
        PreviewConfig.displayPatch = {};
        PreviewConfig.statePatch = {};
        PreviewConfig.appliedSettings = null;
        plasmoid = null;
    }

    function test_brand_replaces_the_spend_header_gap() {
        open();
        verify(!named("brandMark").visible);
        verify(!named("brandName").visible);
        const withSpend = named("refreshButton").parent.implicitHeight;
        cleanup();
        open({
            show_spend: false
        });
        verify(named("brandMark").visible);
        compare(named("brandName").text, "Headroom");
        compare(named("refreshButton").parent.implicitHeight, withSpend);
        verify(named("brandMark").x < named("refreshButton").x);
    }

    function test_breakdown_follows_show_breakdown() {
        open();
        tryVerify(() => named("spendBreakdown")?.visible ?? false, settleMs);
        cleanup();
        open({
            show_breakdown: false
        });
        verify(named("spendBreakdown") !== null);
        verify(!named("spendBreakdown").visible);
    }

    function test_sheen_runs_only_while_expanded() {
        open();
        tryVerify(() => allNamed("meterSheen").some(item => item.visible), settleMs + 6000);
        host().expanded = false;
        tryVerify(() => allNamed("meterSheen").every(item => !item.visible), settleMs);
    }

    function test_sheen_ticks_once_per_cycle_and_rests() {
        open();
        const full = findAll(plasmoid, item => item.sheenRunning !== undefined, [])[0];
        tryVerify(() => allNamed("meterSheen").some(item => item.visible), settleMs + 6000);
        const cycle = full.sheen;
        tryVerify(() => allNamed("meterSheen").every(item => !item.visible), 2000);
        wait(1000);
        compare(full.sheen, cycle);
        verify(allNamed("meterSheen").every(item => !item.visible));
        host().expanded = false;
        tryCompare(full, "sheen", 0);
    }

    function test_cli_login_sign_in_runs_accounts_login() {
        const raw = rawSample();
        const target = raw.accounts.find(account => account.id === signedOutId);
        target.recovery = {
            action: "cli_login",
            command: "claude auth login --claudeai",
            account_id: signedOutId
        };
        open({}, {
            accounts: raw.accounts
        });
        const section = sections().find(item => item.account.id === signedOutId);
        tryVerify(() => JSON.stringify(section.notices[0].actions.map(action => action.kind)) === '["signin","retry","copy"]', settleMs);
        const button = findAll(section, item => item instanceof SmallButton && item.text === "Sign in", [])[0];
        verify(button !== undefined);
        verify(button.primary);
        button.clicked();
        const runner = findAll(plasmoid, item => item.engine === "executable", [])[0];
        compare(runner.connectedSources, [Commands.loginAccountCommand(signedOutId, "Press Enter to close this window")]);
    }

    function test_hover_keeps_card_heights() {
        open();
        const linked = () => findAll(plasmoid, item => item.headerLinks !== undefined && item.headerLinks.length > 0, []);
        tryVerify(() => linked().length > 0, settleMs);
        const header = linked()[0];
        const card = sections().find(item => findAll(item, candidate => candidate === header, []).length > 0);
        const before = [header.height, card.height];
        mouseMove(header, header.width / 2, header.height / 2);
        mouseMove(header, header.width / 2 + 2, header.height / 2);
        tryVerify(() => header.revealed, settleMs);
        compare([header.height, card.height], before);
        mouseMove(suite, 1, 1);
        tryVerify(() => !header.revealed, settleMs);
        compare([header.height, card.height], before);
    }

    function test_big_token_total_fits_inside_the_ring() {
        const donut = createTemporaryObject(donutComponent, suite) as Donut;
        const value = findAll(donut, item => item.objectName === "ringValue", [])[0];
        compare(value.text, "123T");
        verify(donut.fit < 1);
        const caption = findAll(donut, item => item.objectName === "ringCaption", [])[0];
        const width = Math.max(value.implicitWidth, caption.implicitWidth);
        const height = value.implicitHeight + caption.implicitHeight;
        verify(Math.hypot(width / 2, height / 2) <= donut.geometry.inner + 1);
    }

    width: 800
    height: 1400
    visible: true
    when: windowShown

    Component {
        id: plasmoidComponent

        Loader {
            source: "../package/contents/ui/main.qml"
        }
    }

    Component {
        id: donutComponent

        Donut {
            unit: "tokens"
            lang: "en"
            size: 48
            period: SpendState.parsePeriod({
                cost_usd_micros: 1000,
                total_tokens: 123456789012345,
                by_provider: [
                    {
                        provider: "codex",
                        cost_usd_micros: 1000,
                        total_tokens: 123456789012345
                    }
                ]
            })
        }
    }
}
