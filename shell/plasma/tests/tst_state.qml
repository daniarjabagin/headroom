import QtQuick
import QtTest
import "../package/contents/ui/logic/Account.js" as Account
import "../package/contents/ui/logic/I18n.js" as I18n
import "../package/contents/ui/logic/Panel.js" as Panel
import "../package/contents/ui/logic/Registry.js" as Registry
import "../package/contents/ui/logic/State.js" as State
import "../package/contents/ui/logic/Summary.js" as Summary

TestCase {
    readonly property date now: new Date("2026-09-23T10:00:00Z")
    readonly property var optionalSampleKeys: ["models_other", "combined", "account_count", "combine_accounts"]

    function read(relative) {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl(relative), false);
        request.send();
        return request.responseText;
    }

    function sample() {
        return State.parseState(read("../dev/sample-state.json"));
    }

    function providers() {
        return Registry.parseRegistry(read("../dev/sample-providers.json"));
    }

    function daemonJson() {
        return read("../../../crates/headroom-daemon/src/state/snapshots/state_full.json");
    }

    function keyMismatches(sample, daemon, path) {
        if (Array.isArray(sample) && Array.isArray(daemon))
            return sample.length === 0 || daemon.length === 0 ? [] : keyMismatches(sample[0], daemon[0], `${path}[0]`);
        const isObject = value => value !== null && typeof value === "object" && !Array.isArray(value);
        if (!isObject(sample) || !isObject(daemon) || path.endsWith(".hidden_windows"))
            return [];
        const sampleKeys = Object.keys(sample).filter(key => !optionalSampleKeys.includes(key) || key in daemon).sort().join(",");
        const daemonKeys = Object.keys(daemon).sort().join(",");
        if (sampleKeys !== daemonKeys)
            return [`${path}: ${sampleKeys} vs ${daemonKeys}`];
        return Object.keys(daemon).reduce((found, key) => found.concat(keyMismatches(sample[key], daemon[key], `${path}.${key}`)), []);
    }

    function test_sample_accounts() {
        const state = sample();
        compare(state.accounts.length, 7);
        compare(state.headline, {
            accountId: "codex:1a2b3c4d5e6f",
            windowId: "session",
            provider: "codex",
            providerName: "Codex",
            accountLabel: "work",
            windowLabel: "Session",
            usedPercent: 38,
            remainingPercent: 62,
            tone: "good",
            combined: false,
            accountCount: 1
        });
        compare(state.nextRefreshAt.toISOString(), "2026-09-23T10:03:10.000Z");
        compare(state.offline, false);
        const personal = state.accounts[1];
        compare(personal.owner, "headroom");
        compare(state.accounts[0].owner, "cli");
        compare(personal.status, "stale");
        compare(personal.windows[1].pace.severity, "running_out");
        compare(personal.windows[1].pace.sparePercent, null);
        compare(personal.windows[0].pace.sparePercent, 4);
        compare(personal.windows[0].usedPercent, 71);
        compare(personal.windows[0].hidden, false);
        compare(personal.usage, null);
        compare(state.accounts[0].usage.provider, "codex");
        compare(state.accounts[0].balances[0].usdMicros, 12500000);
        compare(state.accounts[2].balances[0].unit, "requests");
        compare(state.accounts[3].status, "signed_out");
        compare(state.accounts[4].status, "no_subscription");
        compare(state.accounts[4].error, {
            kind: "no_subscription",
            message: "the account has no active Codex plan"
        });
    }

    function test_sample_display() {
        const display = sample().display;
        compare(display.theme, "system");
        compare(display.valueMode, "left");
        compare(display.translucent, false);
        compare(display.hiddenWindows, {});
    }

    function test_sample_spend() {
        const state = sample();
        compare(state.accounts[2].usage.last30Days.unpricedModels, ["claude-next"]);
        compare(state.spend.today.costMicros, 23675400);
        compare(state.spend.today.providers.map(spend => spend.provider), ["codex", "claude", "opencode", "cline"]);
        compare(state.spend.today.providers.map(spend => spend.providerName), ["Codex", "Claude", "OpenCode", "Cline"]);
        compare(state.spend.today.providers[0].models[0].model, "gpt-5.5");
        compare(state.spend.last30Days.partial, true);
        const totals = state.accounts[2].usage.last30Days;
        compare(totals.models.length, 5);
        compare(totals.modelsOther, {
            count: 2,
            totalTokens: 2182045,
            costMicros: 964000,
            partial: true
        });
        compare(state.accounts[0].usage.today.modelsOther, null);
        compare(state.spend.last30Days.providers[1].modelsOther.count, 2);
    }

    function test_daemon_snapshot() {
        const json = daemonJson();
        const state = State.parseState(json);
        compare(state.spend.today.costMicros, 12400);
        compare(state.spend.yesterday.partial, true);
        compare(state.accounts[0].windows[1].pace.sparePercent, 47.5);
        compare(state.accounts[1].error.kind, "sign_in_expired");
        compare(state.accounts[2].hidden, true);
        compare(keyMismatches(JSON.parse(read("../dev/sample-state.json")), JSON.parse(json), "$"), []);
    }

    function test_hidden_windows() {
        const raw = JSON.parse(read("../dev/sample-state.json"));
        raw.accounts[0].windows[1].hidden = true;
        raw.display.hidden_windows = {
            "codex:1a2b3c4d5e6f": ["weekly", "weekly", ""]
        };
        const state = State.parseState(JSON.stringify(raw));
        compare(State.shownWindows(state.accounts[0]).map(window => window.id), ["session"]);
        compare(state.display.hiddenWindows, {
            "codex:1a2b3c4d5e6f": ["weekly"]
        });
    }

    function test_edge_states() {
        for (const json of ["{", "{\"version\": 2}", "[]"]) {
            try {
                State.parseState(json);
                fail(`expected a state error for ${json}`);
            } catch (error) {
                verify(State.isStateError(error));
            }
        }
        try {
            State.parseState("{\"version\": 2}");
            fail("expected a state error");
        } catch (error) {
            compare(error.message, "Headroom service speaks state version 2, expected 1");
            compare(I18n.errorText("ru", error), "Служба Headroom использует версию состояния 2, ожидалась 1");
        }
        const bare = State.parseState("{\"version\": 1}");
        compare(bare.accounts, []);
        compare(bare.spend, null);
        compare(bare.headline, null);
        compare(bare.display.language, "system");
    }

    function test_order_and_display_patches() {
        const state = sample();
        const ids = state.accounts.map(account => account.id);
        const reordered = State.withOrder(state, [ids[2], ids[0]]);
        compare(reordered.accounts.map(account => account.id), [ids[2], ids[0], ids[1]].concat(ids.slice(3)));
        compare(State.withDisplay(state, {
            valueMode: "used"
        }).display.valueMode, "used");
        compare(state.display.valueMode, "left");
    }

    function test_headline() {
        const state = sample();
        compare(State.headlineWindow(state).label, "Session");
        compare(State.isHeadlineStale(state), false);
        compare(Panel.headlinePercent(state.headline, "used"), 38);
        compare(Panel.headlinePercent(state.headline, "left"), 62);
        compare(Summary.tooltip("en", {
            kind: "ready",
            state
        }, now), {
            main: "Codex: work · Session",
            sub: "62% left · Resets in 2h 41m"
        });
        compare(Summary.tooltip("ru", {
            kind: "ready",
            state: State.withDisplay(state, {
                valueMode: "used"
            })
        }, now), {
            main: "Codex: work · Сессия",
            sub: "Использовано 38% · Сброс через 2 ч 41 мин"
        });
        compare(Summary.tooltip("en", {
            kind: "unavailable"
        }, now).sub, "Service not running");
    }

    function test_live_clock() {
        const state = sample();
        verify(!State.needsLiveClock(state, now));
        verify(State.needsLiveClock(state, new Date("2026-09-23T12:00:00Z")));
    }

    function test_footer_line() {
        const state = sample();
        compare(Summary.footerLine("en", {
            kind: "ready",
            state
        }, now).text, "Next update in 3m");
        compare(Summary.footerLine("ru", {
            kind: "ready",
            state
        }, now).text, "Обновление через 3 мин");
        state.offline = true;
        verify(Summary.footerLine("en", {
            kind: "ready",
            state
        }, now).notice);
        state.accounts[0].status = "refreshing";
        state.offline = false;
        compare(Summary.footerLine("en", {
            kind: "ready",
            state
        }, now).busy, true);
    }

    function test_account_notices() {
        const state = sample();
        compare(Account.statusSlot(state.accounts[1], false), "outdated");
        compare(Account.statusSlot(state.accounts[2], false), "warning");
        compare(Account.notices("en", state.accounts[2], false, providers())[0].title, "Couldn't refresh Claude");
        compare(Account.notices("ru", state.accounts[2], false, providers())[0].title, "Не удалось обновить Claude");
        compare(Account.notices("en", state.accounts[1], false, providers())[0].kind, "warning");
        const signedOut = Account.notices("en", state.accounts[3], false, providers());
        compare(signedOut.length, 1);
        compare(signedOut[0].detail, "Sign in again through Headroom (Settings → Accounts → Add Account) or remove the account.");
        compare(signedOut[0].actions, [
            {
                kind: "signin",
                label: "Sign in…",
                value: "claude"
            },
            {
                kind: "retry",
                label: "Retry",
                value: state.accounts[3].id,
                busy: false
            }
        ]);
        compare(Account.notices("ru", state.accounts[3], false, providers())[0].actions[0].label, "Войти…");
        compare(Account.notices("en", state.accounts[3], false, [])[0].actions.map(action => action.kind), ["settings", "retry"]);
        const offline = Object.assign({}, state.accounts[2], {
            error: {
                kind: "network",
                message: "offline"
            }
        });
        compare(Account.statusSlot(offline, true), "outdated");
        compare(Account.notices("en", offline, true, providers()), []);
    }

    function test_new_provider_notices() {
        const zai = sample().accounts[5];
        const signedOut = Object.assign({}, zai, {
            status: "signed_out"
        });
        const notice = Account.notices("en", signedOut, false, providers())[0];
        compare(notice.title, "Signed out of Z.ai");
        compare(notice.detail, "Sign in again through Headroom (Settings → Accounts → Add Account) or remove the account.");
        compare(notice.actions.map(action => action.kind), ["signin", "retry"]);
        for (const kind of ["unsupported", "no_provider"]) {
            const failed = Object.assign({}, zai, {
                status: "error",
                error: {
                    kind,
                    message: "not available"
                }
            });
            compare(Account.notices("en", failed, false, providers())[0].actions, []);
        }
    }

    function test_signed_out_retry_shows_progress() {
        const signedOut = sample().accounts[3];
        const retrying = Object.assign({}, signedOut, {
            status: "refreshing"
        });
        const notices = Account.notices("en", retrying, false, providers());
        compare(notices.map(notice => notice.title), ["Signed out of Claude"]);
        compare(notices[0].actions.map(action => [action.kind, action.busy ?? false]), [["signin", false], ["retry", true]]);
        compare(Account.statusSlot(retrying, false), "refreshing");
        verify(!Account.showsQuotas(retrying));
        const refreshingHealthy = Object.assign({}, sample().accounts[0], {
            status: "refreshing"
        });
        compare(Account.notices("en", refreshingHealthy, false, providers()), []);
    }

    function test_signed_out_without_terminal_sign_in_opens_settings() {
        const opencode = Object.assign({}, sample().accounts[5], {
            provider: "opencode",
            providerName: "OpenCode",
            status: "signed_out"
        });
        const actions = Account.notices("en", opencode, false, providers())[0].actions;
        compare(actions.map(action => action.kind), ["settings", "retry"]);
        compare(actions[0].label, "Sign in…");
    }

    function test_removal_text_by_owner() {
        const state = sample();
        const headroom = Account.removal("en", state.accounts[1]);
        compare(headroom.confirmation, "Headroom deletes the sign-in it created for this account. The account itself is not affected.");
        const cli = Account.removal("en", state.accounts[3]);
        compare(cli.confirmation, "Headroom will stop showing this account. The Claude CLI stays signed in; you can sign in again through Headroom.");
        compare(cli.subtitle, "Stops showing this account; the Claude CLI stays signed in");
        compare(Account.removal("ru", state.accounts[3]).confirmation, "Headroom перестанет показывать этот аккаунт. Вход в CLI Claude сохранится; войти снова можно через Headroom.");
    }

    function test_money_balances() {
        const state = State.parseState(JSON.stringify({
            version: 1,
            accounts: [
                {
                    id: "deepseek:1",
                    provider: "deepseek",
                    balances: [
                        {
                            id: "balance_cny",
                            label: "Balance",
                            kind: "money",
                            currency: "CNY",
                            micros: 12500000
                        },
                        {
                            id: "balance_usd",
                            label: "Balance",
                            kind: "money",
                            currency: "USD",
                            micros: -3000000
                        },
                        {
                            id: "bad",
                            label: "Bad",
                            kind: "money",
                            currency: "yuan",
                            micros: 1
                        },
                        {
                            id: "future",
                            label: "Future",
                            kind: "gems",
                            micros: 1
                        }
                    ]
                }
            ]
        }));
        const [cny, usd, bad, future] = state.accounts[0].balances;
        compare([cny.kind, cny.currency, cny.micros, cny.usdMicros], ["money", "CNY", 12500000, null]);
        compare([usd.currency, usd.micros], ["USD", -3000000]);
        compare([bad.kind, bad.currency], ["money", null]);
        compare([future.kind, future.currency, future.micros], [null, null, null]);
    }

    function test_balance_only_account_opens_extras() {
        const state = sample();
        verify(Account.extrasAlwaysOpen(state.accounts[6]));
        verify(!Account.extrasAlwaysOpen(state.accounts[0]));
        verify(!Account.extrasAlwaysOpen(state.accounts[5]));
        verify(!Account.extrasAlwaysOpen(state.accounts[3]));
    }

    function test_provider_names() {
        const state = sample();
        compare(state.accounts.map(account => account.providerName), ["Codex", "Codex", "Claude", "Claude", "Codex", "Z.ai", "OpenRouter"]);
        compare(state.accounts[5].windows.map(window => window.label), ["Session", "Web searches"]);
        compare(state.accounts[6].balances[0].usdMicros, 23750000);
        compare(State.showsName(state.accounts[5], state.accounts), false);
        const raw = JSON.parse(read("../dev/sample-state.json"));
        delete raw.accounts[5].provider_name;
        delete raw.headline.provider_name;
        const bare = State.parseState(JSON.stringify(raw));
        compare(bare.accounts[5].providerName, "zai");
        compare(bare.headline.providerName, "codex");
    }

    function test_no_subscription_notice() {
        const account = sample().accounts[4];
        compare(Account.statusSlot(account, false), "");
        verify(!Account.showsQuotas(account));
        verify(!Account.hasExtras(account, sample().display));
        compare(Account.notices("en", account, false, providers()), [
            {
                kind: "warning",
                title: "No active subscription",
                detail: "Limits aren't available for this account. Renew the plan or sign in with another account.",
                note: "the account has no active Codex plan",
                actions: [
                    {
                        kind: "retry",
                        label: "Retry",
                        value: "codex:3c4d5e6f7a8b",
                        busy: false
                    }
                ]
            }
        ]);
        const russian = Account.notices("ru", account, false, providers())[0];
        compare(russian.title, "Подписка неактивна");
        compare(russian.detail, "Данные о лимитах недоступны. Продлите подписку или войдите в другой аккаунт.");
        const bare = Object.assign({}, account, {
            error: {
                kind: "no_subscription",
                message: "no_subscription"
            }
        });
        compare(Account.notices("en", bare, false, providers())[0].note, "");
    }

    function test_no_subscription_needs_no_live_clock() {
        const state = sample();
        const account = Object.assign({}, state.accounts[0], {
            status: "no_subscription"
        });
        const only = Object.assign({}, state, {
            accounts: [account]
        });
        verify(!State.needsLiveClock(only, new Date(account.windows[0].resetsAt.getTime() - 60000)));
        verify(State.needsLiveClock(Object.assign({}, only, {
            accounts: [state.accounts[0]]
        }), new Date(account.windows[0].resetsAt.getTime() - 60000)));
    }

    function test_account_sections() {
        const state = sample();
        const display = state.display;
        const hiddenSpend = Object.assign({}, display, {
            showAccountSpend: false,
            showTrend: false
        });
        verify(Account.hasExtras(state.accounts[0], display));
        verify(Account.hasExtras(state.accounts[0], hiddenSpend));
        verify(!Account.hasExtras(state.accounts[3], display));
        verify(Account.showsTrend(state.accounts[0], display));
        verify(!Account.showsTrend(state.accounts[0], hiddenSpend));
        compare(Account.spendRows("en", state.accounts[0], hiddenSpend), []);
        const rows = Account.spendRows("ru", state.accounts[0], display);
        compare(rows.map(row => row.title), ["Сегодня", "Вчера", "30 дней"]);
        compare(rows[0].breakdownTitle, "Сегодня · Codex");
    }
}
