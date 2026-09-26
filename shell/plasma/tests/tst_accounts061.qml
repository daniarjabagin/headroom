import QtQuick
import QtTest
import org.kde.plasma.workspace.dbus as DBus
import headroom.preview
import "../package/contents/ui"
import "../package/contents/ui/logic/AccountList.js" as AccountList
import "../package/contents/ui/logic/Commands.js" as Commands
import "../package/contents/ui/logic/State.js" as State

TestCase {
    id: suite

    readonly property int settleMs: 3000
    readonly property int replyDrainMs: 100
    readonly property string expiredId: "claude:5e4d3c2b1a0f"
    property Loader root: null

    function findAll(object, predicate, found) {
        if (predicate(object))
            found.push(object);
        for (const child of object.data ?? [])
            findAll(child, predicate, found);
        return found;
    }

    function named(name) {
        return findAll(root, item => item.objectName === name && item.visible, [])[0] ?? null;
    }

    function allNamed(name) {
        return findAll(root, item => item.objectName === name, []);
    }

    function page(): ConfigAccounts {
        return root.item as ConfigAccounts;
    }

    function calls(member) {
        return DBus.SessionBus.messages.filter(message => message.member === member);
    }

    function read(relative) {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl(relative), false);
        request.send();
        return request.responseText;
    }

    function sample() {
        return State.parseState(read("../dev/sample-state.json"));
    }

    function load(statePatch) {
        PreviewConfig.scenario = "ready";
        PreviewConfig.displayPatch = {
            language: "en"
        };
        PreviewConfig.appliedSettings = null;
        PreviewConfig.statePatch = statePatch ?? {};
        root = createTemporaryObject(pageComponent, suite, {
            source: "../package/contents/ui/ConfigAccounts.qml"
        }) as Loader;
        tryVerify(() => page() !== null && page().ready, settleMs);
        wait(replyDrainMs);
        DBus.SessionBus.messages = [];
    }

    function cleanup() {
        wait(replyDrainMs);
        PreviewConfig.appliedSettings = null;
        PreviewConfig.displayPatch = {};
        PreviewConfig.statePatch = {};
        root = null;
    }

    function test_rows_group_accounts_by_provider() {
        const accounts = sample().accounts;
        const rows = AccountList.rows(accounts);
        const headings = rows.filter(row => row.kind === "provider").map(row => row.provider);
        compare(headings, [...new Set(accounts.map(account => account.provider))]);
        compare(rows.filter(row => row.kind === "account").length, accounts.length);
        const codex = rows.findIndex(row => row.key === "provider:codex");
        compare(rows[codex + 1].account.provider, "codex");
        compare(rows[codex + 2].account.provider, "codex");
    }

    function test_status_and_sign_in_target() {
        const accounts = sample().accounts;
        const expired = accounts.find(account => account.id === expiredId);
        compare(AccountList.status("en", expired), {
            kind: "error",
            text: "Signed out"
        });
        compare(AccountList.status("ru", expired).text, "Выполнен выход");
        compare(AccountList.status("en", accounts[0]).kind, "");
        compare(AccountList.signInTarget(Object.assign({}, expired, {
            recovery: {
                action: "sign_in",
                accountId: null
            }
        })), expiredId);
        compare(AccountList.signInTarget(Object.assign({}, expired, {
            recovery: {
                action: "cli_login",
                command: "claude",
                accountId: "claude:77"
            }
        })), "claude:77");
        compare(AccountList.signInTarget(Object.assign({}, expired, {
            recovery: {
                action: "retry"
            },
            owner: "cli"
        })), null);
        compare(AccountList.selected(accounts, "missing").id, accounts[0].id);
        compare(AccountList.selected([], "missing"), null);
    }

    function test_selecting_a_row_shows_its_details() {
        load();
        const rows = allNamed("accountListRow");
        compare(rows.length, page().accounts.length);
        verify(rows[0].current);
        compare(named("detailTitle").text, rows[0].Accessible.name);
        mouseClick(rows[2]);
        tryVerify(() => rows[2].current && !rows[0].current, settleMs);
        compare(named("detailTitle").text, rows[2].Accessible.name);
    }

    function test_detail_star_move_and_visibility() {
        load();
        const ids = page().accounts.map(account => account.id);
        mouseClick(named("accountStar"));
        tryVerify(() => (PreviewConfig.currentSettings().display.starred_accounts ?? []).includes(ids[0]), settleMs);
        verify(!named("moveUp").enabled);
        mouseClick(named("moveDown"));
        tryVerify(() => calls("SetAccountOrder").length === 1, settleMs);
        compare(calls("SetAccountOrder")[0].arguments[0].slice(0, 2), [ids[1], ids[0]]);
        mouseClick(named("accountVisible"));
        tryVerify(() => calls("SetAccountHidden").length === 1, settleMs);
        compare(calls("SetAccountHidden")[0].arguments, [ids[0], true]);
    }

    function test_sign_in_again_runs_accounts_login() {
        const raw = JSON.parse(read("../dev/sample-state.json"));
        raw.accounts.find(account => account.id === expiredId).recovery = {
            action: "cli_login",
            command: "claude auth login --claudeai",
            account_id: expiredId
        };
        load({
            accounts: raw.accounts
        });
        verify(named("signInAgainRow") === null);
        page().select(expiredId);
        tryVerify(() => named("signInAgainRow") !== null, settleMs);
        mouseClick(named("signInAgain"));
        const runner = findAll(root, item => item.engine === "executable", [])[0];
        compare(runner.connectedSources, [Commands.loginAccountCommand(expiredId, "Press Enter to close this window")]);
    }

    function test_add_button_opens_the_add_pane() {
        load();
        verify(named("addAccountPane") === null);
        mouseClick(named("addAccountButton"));
        tryVerify(() => named("addAccountPane") !== null, settleMs);
        verify(named("accountDetail") === null);
        mouseClick(allNamed("accountListRow")[1]);
        tryVerify(() => named("accountDetail") !== null && named("addAccountPane") === null, settleMs);
    }

    width: 900
    height: 1600
    visible: true
    when: windowShown

    Component {
        id: pageComponent

        Loader {
            width: 860
            height: 1600
        }
    }
}
