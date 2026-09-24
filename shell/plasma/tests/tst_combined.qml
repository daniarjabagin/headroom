import QtQuick
import QtTest
import "../package/contents/ui/logic/Combined.js" as Combined
import "../package/contents/ui/logic/Format.js" as Format
import "../package/contents/ui/logic/Order.js" as Order
import "../package/contents/ui/logic/Quota.js" as Quota
import "../package/contents/ui/logic/Settings.js" as Settings
import "../package/contents/ui/logic/State.js" as State

TestCase {
    name: "Combined"

    readonly property date now: new Date("2026-09-23T10:00:00Z")

    function accountWindow(used, resetsAt, tone, even) {
        return {
            id: "session",
            label: "Session",
            used_percent: used,
            remaining_percent: 100 - used,
            resets_at: resetsAt,
            tone,
            pace: {
                severity: "healthy",
                even_pace_percent: even,
                projected_percent: 60,
                spare_percent: 40
            }
        };
    }

    function account(id, label, plan, status, windows) {
        return {
            id,
            provider: id.split(":")[0],
            provider_name: "Codex",
            label,
            plan,
            status,
            windows
        };
    }

    function segment(accountId, label, used, resetsAt, tone) {
        return {
            account_id: accountId,
            label,
            remaining_percent: 100 - used,
            used_percent: used,
            resets_at: resetsAt,
            tone
        };
    }

    function group() {
        return {
            provider: "codex",
            provider_name: "Codex",
            account_ids: ["codex:work", "codex:personal"],
            accounts: [
                {
                    account_id: "codex:work",
                    label: "work",
                    plan: "Pro"
                },
                {
                    account_id: "codex:personal",
                    label: "personal",
                    plan: "Plus"
                }
            ],
            windows: [
                {
                    id: "session",
                    label: "Session",
                    capacity_percent: 200,
                    remaining_percent: 145,
                    used_percent: 55,
                    resets_at: "2026-09-23T11:00:00Z",
                    tone: "warning",
                    pace: {
                        severity: "close",
                        even_pace_percent: 80,
                        projected_percent: 180,
                        spare_percent: 20,
                        runs_out_at: null
                    },
                    segments: [segment("codex:work", "work", 5, "2026-09-23T12:41:00Z", "good"), segment("codex:personal", "personal", 50, "2026-09-23T11:00:00Z", "warning")]
                }
            ]
        };
    }

    function sampleState(combine, groups) {
        return State.parseState(JSON.stringify({
            version: 1,
            display: {
                combine_accounts: combine
            },
            headline: {
                account_id: null,
                window: "session",
                provider: "codex",
                provider_name: "Codex",
                account_label: null,
                window_label: "Session",
                used_percent: 27.5,
                remaining_percent: 72.5,
                tone: "warning",
                combined: true,
                account_count: 2
            },
            accounts: [account("claude:main", "main", "Max", "fresh", []), account("codex:work", "work", "Pro", "fresh", [accountWindow(5, "2026-09-23T12:41:00Z", "good", 46)]), account("codex:old", "old", "Plus", "signed_out", []), account("codex:personal", "personal", "Plus", "stale", [accountWindow(50, "2026-09-23T11:00:00Z", "warning", 80)])],
            combined: groups ?? [group()]
        }));
    }

    function kinds(cards) {
        return cards.map(card => [card.kind, card.accountIds]);
    }

    function test_parse() {
        const parsed = sampleState(true);
        compare(parsed.combined[0].windows[0].capacityPercent, 200);
        compare(parsed.combined[0].windows[0].segments[1].accountId, "codex:personal");
        compare([parsed.headline.combined, parsed.headline.accountCount], [true, 2]);
        compare(State.parseState('{"version": 1, "combined": [{"provider": "codex"}]}').combined, []);
    }

    function test_cards() {
        const on = sampleState(true);
        compare(kinds(Combined.cards(on, on.accounts)), [["account", ["claude:main"]], ["combined", ["codex:work", "codex:personal"]], ["account", ["codex:old"]]]);
        const off = sampleState(false);
        compare(Combined.cards(off, off.accounts).length, 4);
        const bare = sampleState(true, []);
        compare(Combined.cards(bare, bare.accounts).length, 4);
        const visible = on.accounts.filter(entry => entry.id !== "codex:work");
        compare(kinds(Combined.cards(on, visible))[2], ["combined", ["codex:personal"]]);
    }

    function test_header() {
        const parsed = sampleState(true);
        const card = Combined.cards(parsed, parsed.accounts)[1];
        const header = Combined.headerAccount("en", card);
        compare([header.providerName, header.plan, header.status], ["Codex", "2 accounts · Pro · Plus", "stale"]);
        compare(Combined.groupDetail("ru", card.group), "2 аккаунта · Pro · Plus");
        compare([1, 2, 5, 21].map(count => Combined.accountCountText("ru", count)), ["1 аккаунт", "2 аккаунта", "5 аккаунтов", "21 аккаунт"]);
    }

    function test_readings() {
        compare(Format.capacityReading("en", 145, 200, "left"), "145% left of 200%");
        compare(Format.capacityReading("en", 55.4, 200, "used"), "55% used of 200%");
        compare(Format.capacityReading("ru", 145, 200, "left"), "Осталось 145% из 200%");
        compare(Format.capacityReading("ru", 55, 200, "used"), "Использовано 55% из 200%");
        compare(Format.capacityReading("en", null, 200, "left"), "—");
        compare(Format.panelCount(2), "×2");
        const window = sampleState(true).combined[0].windows[0];
        compare(Combined.combinedPercent(window, "left"), 145);
        compare(Combined.combinedPercent(window, "used"), 55);
        compare(Combined.combinedPercent(Object.assign({}, window, {
            usedPercent: null
        }), "used"), 55);
    }

    function test_segments() {
        const parsed = sampleState(true);
        const card = Combined.cards(parsed, parsed.accounts)[1];
        const display = Settings.parseDisplay(null);
        const left = Combined.segments(card.group.windows[0], card.members, display);
        compare(left.map(entry => [entry.fraction, entry.tone]), [[0.95, "good"], [0.5, "warning"]]);
        fuzzyCompare(left[0].tick, 0.54, 1e-9);
        fuzzyCompare(left[1].tick, 0.2, 1e-9);
        const used = Combined.segments(card.group.windows[0], card.members, Object.assign({}, display, {
            valueMode: "used",
            showForecast: false
        }));
        compare(used.map(entry => [entry.fraction, entry.tick]), [[0.05, null], [0.5, 0.8]]);
    }

    function test_breakdown() {
        const parsed = sampleState(true);
        const card = Combined.cards(parsed, parsed.accounts)[1];
        compare(Combined.breakdown("en", card.group.windows[0], card.members, now, Settings.parseDisplay(null)), "work: 95% left · Resets in 2h 41m\npersonal: 50% left · Resets in 1h 0m");
    }

    function test_forecast() {
        const window = sampleState(true).combined[0].windows[0];
        const display = Settings.parseDisplay(null);
        compare(Format.forecastText("en", window, now, display), "At this pace: ~20% of 200% left at reset");
        compare(Format.forecastText("en", window, now, Object.assign({}, display, {
            valueMode: "used"
        })), "At this pace: ~180% of 200% used at reset");
        const over = Object.assign({}, window, {
            pace: Object.assign({}, window.pace, {
                severity: "running_out",
                sparePercent: null
            })
        });
        compare(Format.forecastText("en", over, now, display), "At this pace: runs out before reset");
        compare(Quota.paceNote("en", over, now, false).text, "Over pace");
    }

    function test_setting_and_order() {
        compare(Settings.parseDisplay(null).combineAccounts, false);
        compare(Settings.parseDisplay({
            combine_accounts: true
        }).combineAccounts, true);
        compare(Settings.displayPatch({
            combineAccounts: true
        }), {
            display: {
                combine_accounts: true
            }
        });
        compare(Order.reorderedGroups(["a", "b", "c", "d"], [["a"], ["b", "d"], ["c"]], 1, 0), ["b", "d", "a", "c"]);
    }
}
