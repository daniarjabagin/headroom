import QtQuick
import QtTest
import headroom.preview
import "../package/contents/ui"
import "../package/contents/ui/logic/Account.js" as Account
import "../package/contents/ui/logic/Recovery.js" as Recovery
import "../package/contents/ui/logic/Registry.js" as Registry
import "../package/contents/ui/logic/State.js" as State

TestCase {
    id: suite

    readonly property int settleMs: 3000
    readonly property int replyDrainMs: 100
    readonly property string errorAccountId: "claude:0a1b2c3d4e5f"
    property Item plasmoid: null

    function read(relative) {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl(relative), false);
        request.send();
        return request.responseText;
    }

    function providers() {
        return Registry.parseRegistry(read("../dev/sample-providers.json"));
    }

    function account(status, error, recovery) {
        return {
            id: "claude:5e4d3c2b1a0f",
            provider: "claude",
            providerName: "Claude",
            owner: "cli",
            status,
            error,
            recovery,
            notices: [],
            windows: [],
            balances: [],
            usage: null
        };
    }

    function signedOutError() {
        return {
            kind: "sign_in_expired",
            message: "sign-in expired"
        };
    }

    function kinds(notice) {
        return notice.actions.map(action => [action.kind, action.busy ?? false]);
    }

    function test_parse_recovery_is_tolerant() {
        compare(Recovery.parseRecovery(undefined), null);
        compare(Recovery.parseRecovery(null), null);
        compare(Recovery.parseRecovery("retry"), null);
        compare(Recovery.parseRecovery({
            action: "retry"
        }), {
            action: "retry"
        });
        compare(Recovery.parseRecovery({
            action: "sign_in",
            account_id: "grok:1a2b"
        }), {
            action: "sign_in",
            accountId: "grok:1a2b"
        });
        compare(Recovery.parseRecovery({
            action: "cli_login",
            command: " codex login "
        }), {
            action: "cli_login",
            command: "codex login",
            accountId: null
        });
        compare(Recovery.parseRecovery({
            action: "cli_login",
            command: "claude auth login",
            account_id: "claude:5e4d"
        }).accountId, "claude:5e4d");
        compare(Recovery.parseRecovery({
            action: "cli_login"
        }), null);
        compare(Recovery.parseRecovery({
            action: "teleport"
        }), null);
    }

    function test_state_carries_recovery() {
        const state = State.parseState(read("../dev/sample-state.json"));
        compare(state.accounts.find(entry => entry.id === errorAccountId).recovery, {
            action: "retry"
        });
        compare(state.accounts[0].recovery, null);
        const legacy = State.parseState(JSON.stringify({
            version: 1,
            accounts: [
                {
                    id: "codex:1",
                    provider: "codex",
                    status: "error",
                    error: {
                        kind: "account_changed",
                        message: "another account is signed in"
                    }
                }
            ]
        }));
        compare(legacy.accounts[0].recovery, null);
        compare(kinds(Account.notices("en", legacy.accounts[0], false, providers())[0]), [["retry", false]]);
    }

    function test_cli_login_offers_copy_command() {
        const signedOut = account("signed_out", signedOutError(), {
            action: "cli_login",
            command: "claude auth login --claudeai"
        });
        const notice = Account.notices("en", signedOut, false, providers())[0];
        compare(notice.title, "Signed out of Claude");
        compare(notice.detail, "Run `claude auth login --claudeai` in a terminal — Headroom picks it up automatically.");
        compare(kinds(notice), [["copy", false], ["retry", false]]);
        compare(notice.actions[0].label, "Copy command");
        compare(notice.actions[0].value, "claude auth login --claudeai");
        compare(Recovery.buttonLabel(notice.actions[0], ""), "Copy command");
        compare(Recovery.buttonLabel(notice.actions[0], "claude auth login --claudeai"), "Copied");
        compare(Recovery.buttonLabel(notice.actions[1], "claude auth login --claudeai"), "Retry");
        const russian = Account.notices("ru", signedOut, false, providers())[0];
        compare(russian.detail, "Выполните `claude auth login --claudeai` в терминале — Headroom подхватит вход сам.");
        compare(russian.actions.map(action => action.label), ["Скопировать команду", "Повторить"]);
        compare(russian.actions[0].doneLabel, "Скопировано");
    }

    function test_cli_login_with_account_signs_in_through_headroom() {
        const signedOut = account("signed_out", signedOutError(), {
            action: "cli_login",
            command: "claude auth login --claudeai",
            accountId: "claude:5e4d3c2b1a0f"
        });
        const notice = Account.notices("en", signedOut, false, providers())[0];
        compare(kinds(notice), [["signin", false], ["retry", false], ["copy", false]]);
        compare(notice.actions[0].label, "Sign in");
        compare(notice.actions[0].value, "claude:5e4d3c2b1a0f");
        verify(notice.actions[0].primary);
        compare(notice.actions[2].value, "claude auth login --claudeai");
        compare(Account.notices("ru", signedOut, false, providers())[0].actions.map(action => action.label), ["Войти", "Повторить", "Скопировать команду"]);
    }

    function test_cli_login_on_error_keeps_message_and_hint() {
        const apiKeyOnly = account("error", {
            kind: "api_key_only",
            message: "signed in with an API key"
        }, {
            action: "cli_login",
            command: "codex login"
        });
        const notice = Account.notices("en", apiKeyOnly, false, providers())[0];
        compare(notice.detail, "signed in with an API key");
        compare(notice.note, "Run `codex login` in a terminal — Headroom picks it up automatically.");
        compare(kinds(notice), [["copy", false], ["retry", false]]);
    }

    function test_sign_in_recovery_opens_the_sign_in_flow() {
        const expired = Object.assign(account("signed_out", signedOutError(), {
            action: "sign_in",
            accountId: "claude:5e4d3c2b1a0f"
        }), {
            owner: "headroom"
        });
        const notice = Account.notices("en", expired, false, providers())[0];
        compare(notice.actions, [
            {
                kind: "signin",
                label: "Sign in again…",
                value: "claude:5e4d3c2b1a0f"
            }
        ]);
        compare(notice.detail, "Headroom's sign-in for this account has expired. Sign in again or remove the account.");
        compare(Account.notices("ru", expired, false, providers())[0].actions[0].label, "Войти снова…");
        compare(Account.notices("en", expired, false, [])[0].actions.map(action => [action.kind, action.value]), [["signin", "claude:5e4d3c2b1a0f"]]);
        const withoutId = Object.assign({}, expired, {
            recovery: {
                action: "sign_in",
                accountId: null
            }
        });
        compare(Account.notices("en", withoutId, false, providers())[0].actions[0].value, expired.id);
    }

    function test_retry_recovery_and_waiting_errors() {
        const failed = account("error", {
            kind: "network",
            message: "connection reset"
        }, {
            action: "retry"
        });
        compare(kinds(Account.notices("en", failed, false, providers())[0]), [["retry", false]]);
        const limited = account("error", {
            kind: "rate_limited",
            message: "slow down"
        }, null);
        compare(Account.notices("en", limited, false, providers())[0].actions, []);
        const signedOutRetry = account("signed_out", signedOutError(), {
            action: "retry"
        });
        compare(kinds(Account.notices("en", signedOutRetry, false, providers())[0]), [["signin", false], ["retry", false]]);
    }

    function limitedWithData(status) {
        return Object.assign(account(status, {
            kind: "rate_limited",
            message: "usage endpoint rate limited by the provider"
        }, null), {
            updatedAt: new Date(2026, 8, 23, 17, 40),
            refresh: {
                mode: "idle",
                intervalSecs: 300,
                nextAt: new Date(2026, 8, 23, 18, 5),
                reason: "hold"
            }
        });
    }

    function test_rate_limit_with_data_is_a_quiet_note() {
        for (const status of ["fresh", "stale", "refreshing"]) {
            const limited = limitedWithData(status);
            compare(Account.notices("en", limited, false, providers()), [], status);
            compare(Account.settledStatus(limited), status);
        }
        compare(Account.statusSlot(limitedWithData("stale"), false), "outdated");
        compare(Account.statusSlot(limitedWithData("fresh"), false), "");
        const limited = limitedWithData("stale");
        compare(Account.rateLimitNote("en", limited, "24h"), "Provider is limiting requests · next try 18:05");
        compare(Account.rateLimitNote("en", limited, "12h"), "Provider is limiting requests · next try 6:05 PM");
        compare(Account.rateLimitNote("ru", limited, "24h"), "Провайдер ограничил запросы · повтор в 18:05");
        const unscheduled = Object.assign({}, limited, {
            refresh: null
        });
        compare(Account.rateLimitNote("en", unscheduled, "24h"), "Provider is limiting requests");
    }

    function test_rate_limit_without_data_keeps_the_error_notice() {
        const limited = Object.assign(limitedWithData("error"), {
            updatedAt: null
        });
        const notices = Account.notices("en", limited, false, providers());
        compare(notices.length, 1);
        compare(notices[0].title, "Couldn't refresh Claude");
        compare(Account.rateLimitNote("en", limited, "24h"), "");
        compare(Account.rateLimitNote("en", account("fresh", null, null), "24h"), "");
    }

    function test_error_notice_stays_while_refreshing() {
        const failed = account("error", {
            kind: "invalid_response",
            message: "HTTP 503"
        }, {
            action: "retry"
        });
        const before = Account.notices("en", failed, false, providers());
        const during = Account.notices("en", Object.assign({}, failed, {
            status: "refreshing"
        }), false, providers());
        compare(during.length, before.length);
        compare(during[0].title, before[0].title);
        compare(during[0].detail, "HTTP 503");
        compare(kinds(during[0]), [["retry", true]]);
        compare(Account.statusSlot(Object.assign({}, failed, {
            status: "refreshing"
        }), false), "refreshing");
    }

    function test_other_notices_stay_while_refreshing() {
        const cliLogin = account("refreshing", signedOutError(), {
            action: "cli_login",
            command: "claude"
        });
        const notice = Account.notices("en", cliLogin, false, providers())[0];
        compare(notice.title, "Signed out of Claude");
        compare(kinds(notice), [["copy", false], ["retry", true]]);
        verify(!Account.showsQuotas(cliLogin));
        const noPlan = account("refreshing", {
            kind: "no_subscription",
            message: "no plan"
        }, null);
        compare(Account.notices("en", noPlan, false, providers())[0].title, "No active subscription");
        compare(kinds(Account.notices("en", noPlan, false, providers())[0]), [["retry", true]]);
        const offline = account("refreshing", {
            kind: "network",
            message: "offline"
        }, {
            action: "retry"
        });
        compare(Account.notices("en", offline, true, providers()), []);
        compare(Account.notices("en", account("refreshing", null, null), false, providers()), []);
    }

    function findAll(object, predicate, found) {
        if (predicate(object))
            found.push(object);
        for (const child of object.data ?? [])
            findAll(child, predicate, found);
        return found;
    }

    function errorSection() {
        return findAll(plasmoid, item => item.account?.id === errorAccountId && item.notices !== undefined && item.windows !== undefined, [])[0];
    }

    function noticeRow(section) {
        return findAll(section, item => item.entry !== undefined && item.copiedValue !== undefined, [])[0];
    }

    function retryButton(row) {
        return findAll(row, item => item.busy !== undefined && item.text === "Retry", [])[0];
    }

    function pushState(scenario) {
        PreviewConfig.scenario = scenario;
        const bridge = findAll(plasmoid, item => item.stateReceived !== undefined && item.openRequested !== undefined, [])[0];
        bridge.stateReceived(PreviewConfig.shiftedState());
    }

    function cleanup() {
        wait(replyDrainMs);
        PreviewConfig.scenario = "ready";
        PreviewConfig.displayPatch = {};
        plasmoid = null;
    }

    function test_error_notice_is_not_rebuilt_across_a_retry() {
        PreviewConfig.scenario = "ready";
        PreviewConfig.displayPatch = {
            language: "en"
        };
        plasmoid = createTemporaryObject(plasmoidComponent, suite) as Item;
        tryVerify(() => errorSection() !== undefined && noticeRow(errorSection()) !== undefined, settleMs);
        const row = noticeRow(errorSection());
        const button = retryButton(row);
        verify(!button.busy);
        pushState("retrying-error");
        tryCompare(errorSection().account, "status", "refreshing");
        verify(noticeRow(errorSection()) === row);
        verify(retryButton(row) === button);
        verify(button.busy);
        pushState("ready");
        tryCompare(errorSection().account, "status", "error");
        verify(noticeRow(errorSection()) === row);
        verify(!button.busy);
    }

    function test_copy_button_copies_and_confirms() {
        const row = createTemporaryObject(noticeComponent, suite);
        const triggered = createTemporaryObject(spyComponent, suite, {
            target: row
        });
        const copy = findAll(row, item => item.busy !== undefined && item.text === "Copy command", [])[0];
        verify(copy !== undefined);
        copy.clicked();
        compare(row.copiedValue, "codex login");
        compare(copy.text, "Copied");
        compare(triggered.count, 0);
        const retry = retryButton(row);
        retry.clicked();
        compare(triggered.count, 1);
        compare(Array.from(triggered.signalArguments[0]), ["retry", "codex:1"]);
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
        id: noticeComponent

        NoticeRow {
            width: 400
            entry: ({
                    kind: "signin",
                    title: "Signed out of Codex",
                    detail: "",
                    note: "",
                    actions: [
                        {
                            kind: "copy",
                            label: "Copy command",
                            doneLabel: "Copied",
                            value: "codex login"
                        },
                        {
                            kind: "retry",
                            label: "Retry",
                            value: "codex:1",
                            busy: false
                        }
                    ]
                })
        }
    }

    Component {
        id: spyComponent

        SignalSpy {
            signalName: "actionTriggered"
        }
    }
}
