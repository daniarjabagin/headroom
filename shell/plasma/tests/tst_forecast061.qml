import QtQuick
import QtTest
import "../package/contents/ui/logic/Format.js" as Format
import "../package/contents/ui/logic/FormatSpend.js" as FormatSpend
import "../package/contents/ui/logic/FormatTime.js" as FormatTime
import "../package/contents/ui/logic/Motion.js" as Motion
import "../package/contents/ui/logic/Quota.js" as Quota
import "../package/contents/ui/logic/RingFit.js" as RingFit
import "../package/contents/ui/logic/Settings.js" as Settings
import "../package/contents/ui/logic/SpendBreakdown.js" as SpendBreakdown
import "../package/contents/ui/logic/SpendState.js" as SpendState
import "../package/contents/ui/logic/State.js" as State

TestCase {
    id: suite

    readonly property var now: new Date("2026-09-26T12:00:00Z")
    readonly property var display: Settings.parseDisplay({
        show_forecast: true
    })

    function window(pace) {
        return State.parseState(JSON.stringify({
            version: 1,
            accounts: [
                {
                    id: "codex:1a",
                    provider: "codex",
                    windows: [
                        {
                            id: "session",
                            used_percent: 40,
                            remaining_percent: 60,
                            resets_at: "2026-09-26T15:00:00Z",
                            tone: "good",
                            pace
                        }
                    ]
                }
            ]
        })).accounts[0].windows[0];
    }

    function test_pace_basis_is_parsed_defensively() {
        compare(window({
            basis: "paused",
            active_left_seconds: 10800
        }).pace.basis, "paused");
        compare(window({
            basis: "paused",
            active_left_seconds: 10800
        }).pace.activeLeftSeconds, 10800);
        compare(window({
            basis: "recent"
        }).pace.basis, "recent");
        compare(window({
            basis: "window"
        }).pace.basis, "window");
        compare(window({
            basis: "sometimes",
            active_left_seconds: -5
        }).pace.basis, null);
        compare(window({
            basis: "paused",
            active_left_seconds: -5
        }).pace.activeLeftSeconds, null);
        compare(window({
            basis: "paused",
            active_left_seconds: "3h"
        }).pace.activeLeftSeconds, null);
        compare(window(null).pace.basis, null);
        compare(window(null).pace.activeLeftSeconds, null);
    }

    function test_paused_forecast_lines() {
        const lasting = window({
            severity: "healthy",
            basis: "paused",
            spare_percent: 20,
            active_left_seconds: 10800
        });
        compare(Format.forecastText("en", lasting, now, display), "Paused · lasts ≈3 h of work");
        compare(Format.forecastText("ru", lasting, now, display), "Пауза · хватит ≈3 ч работы");
        const bare = window({
            severity: "running_out",
            basis: "paused"
        });
        compare(Format.forecastText("en", bare, now, display), "Paused");
        compare(Format.forecastText("ru", bare, now, display), "Пауза");
        compare(Quota.forecast("en", bare, now, Settings.parseDisplay({
            show_forecast: false
        })), null);
    }

    function test_recent_and_window_keep_the_countdown() {
        for (const basis of ["recent", "window", null]) {
            const running = window({
                severity: "running_out",
                basis,
                runs_out_at: "2026-09-26T13:30:00Z"
            });
            compare(Format.forecastText("en", running, now, display), "At this pace: runs out in 1h 30m · resets in 3h 0m");
        }
    }

    function test_paused_note_has_no_countdown() {
        const noForecast = Settings.parseDisplay({
            show_forecast: false
        });
        const paused = window({
            severity: "running_out",
            basis: "paused"
        });
        compare(Quota.paceNote("en", paused, now, false).text, "Over pace");
        const recent = window({
            severity: "running_out",
            basis: "recent",
            runs_out_at: "2026-09-26T13:00:00Z"
        });
        compare(Quota.paceNote("en", recent, now, noForecast.showForecast).text, "Limit in 1h 0m");
    }

    function test_rough_duration() {
        const cases = [[20 * 1000, "1 min", "1 мин"], [25 * 60 * 1000, "25 min", "25 мин"], [3 * 3600 * 1000 + 20 * 60 * 1000, "3 h", "3 ч"], [3 * 3600 * 1000 + 40 * 60 * 1000, "4 h", "4 ч"], [47 * 3600 * 1000, "47 h", "47 ч"], [3 * 86400 * 1000, "3 d", "3 д"]];
        for (const [ms, english, russian] of cases) {
            compare(FormatTime.roughDuration("en", ms), english);
            compare(FormatTime.roughDuration("ru", ms), russian);
        }
    }

    function test_ring_fit_scale() {
        compare(RingFit.scale(60, 2, 30, 20), 1);
        fuzzyCompare(RingFit.scale(60, 2, 80, 20), 28 / Math.hypot(40, 10), 1e-9);
        compare(RingFit.scale(60, 2, 0, 0), 1);
        compare(RingFit.scale(4, 4, 10, 10), 0);
        const fitted = RingFit.scale(52, 3, 70, 30);
        verify(Math.hypot(35 * fitted, 15 * fitted) <= 23 + 1e-9);
    }

    function test_trillions_stay_compact() {
        compare(FormatSpend.compactTokens(1234000000000), "1.2T");
        compare(FormatSpend.compactTokens(987000000000), "987B");
    }

    function test_sheen_timing_and_threshold() {
        compare(Motion.sheenSweepMs(), 1400);
        compare(Motion.sheenCycleMs(), 6400);
        compare(Motion.sheenStartDelayMs(), 600);
        verify(!Motion.sheenShown(0, 0.5));
        verify(Motion.sheenShown(1, 0.5));
        verify(Motion.sheenShown(7, 0.03));
        verify(!Motion.sheenShown(3, 0.02));
    }

    function test_show_breakdown_defaults_on() {
        compare(Settings.parseDisplay({}).showBreakdown, true);
        compare(Settings.parseDisplay({
            show_breakdown: false
        }).showBreakdown, false);
        compare(Settings.parseDisplay({
            show_breakdown: "no"
        }).showBreakdown, true);
        compare(Settings.displayPatch({
            showBreakdown: false
        }), {
            display: {
                show_breakdown: false
            }
        });
    }

    function period(providers) {
        return SpendState.parsePeriod({
            cost_usd_micros: 10000000,
            total_tokens: 1000,
            projects: [
                {
                    project: "/work/a",
                    cost_usd_micros: 10000000,
                    total_tokens: 1000,
                    share_permille: 1000
                }
            ],
            by_provider: providers
        });
    }

    function provider(id, models, other) {
        return {
            provider: id,
            cost_usd_micros: 5000000,
            total_tokens: 500,
            models,
            models_other: other
        };
    }

    function model(name, cost) {
        return {
            model: name,
            total_tokens: 100,
            cost_usd_micros: cost,
            cost_per_mtok_usd_micros: cost * 10000
        };
    }

    function otherRow(result, key) {
        return result.rows.find(row => row.key === key);
    }

    function test_other_rate_comes_from_the_daemon() {
        const five = [model("a", 900), model("b", 800), model("c", 700), model("d", 600), model("e", 500)];
        const merged = period([provider("codex", five, null)]);
        merged.models = SpendState.parsePeriod({
            models: five.map(entry => Object.assign({
                    provider: "codex"
                }, entry))
        }).models;
        merged.modelsOther = SpendState.parsePeriod({
            models_other: {
                count: 3,
                total_tokens: 300,
                cost_usd_micros: 300,
                cost_per_mtok_usd_micros: 1000000
            }
        }).modelsOther;
        compare(otherRow(SpendBreakdown.breakdown("en", merged, "models", "cost_per_mtok"), "model:other").value, "$1.00");
        merged.modelsOther.costPerMtokMicros = null;
        compare(otherRow(SpendBreakdown.breakdown("en", merged, "models", "cost_per_mtok"), "model:other").value, "—");
    }

    function test_older_daemon_shows_provider_lists_without_other_rate() {
        const five = [model("a", 900), model("b", 800), model("c", 700), model("d", 600), model("e", 500)];
        const legacy = period([provider("codex", five, {
                count: 3,
                total_tokens: 300,
                cost_usd_micros: 300
            }), provider("claude", [model("f", 100)], null)]);
        compare(legacy.models, null);
        const result = SpendBreakdown.breakdown("en", legacy, "models", "cost_per_mtok");
        compare(result.rows.map(row => row.key), ["model:codex:a", "model:codex:b", "model:codex:c", "model:codex:d", "model:codex:e", "model:other:codex", "model:claude:f"]);
        compare(otherRow(result, "model:other:codex").value, "—");
        compare(result.caption, "9 models");
    }
}
