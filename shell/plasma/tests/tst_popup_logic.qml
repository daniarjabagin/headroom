import QtQuick
import QtTest
import "../package/contents/ui/logic/Breakdown.js" as Breakdown
import "../package/contents/ui/logic/Collapse.js" as Collapse
import "../package/contents/ui/logic/CompactRow.js" as CompactRow
import "../package/contents/ui/logic/Density.js" as Density
import "../package/contents/ui/logic/Incident.js" as Incident
import "../package/contents/ui/logic/QuickLinks.js" as QuickLinks
import "../package/contents/ui/logic/Settings.js" as Settings
import "../package/contents/ui/logic/Spend.js" as Spend
import "../package/contents/ui/logic/SpendBreakdown.js" as SpendBreakdown
import "../package/contents/ui/logic/SpendUnits.js" as SpendUnits
import "../package/contents/ui/logic/State.js" as State

TestCase {
    name: "PopupLogic"

    readonly property var units: ({
            gridUnit: 18,
            smallSpacing: 4,
            mediumSpacing: 6,
            largeSpacing: 8,
            iconSizes: {
                small: 16
            }
        })
    readonly property var now: new Date(2026, 8, 23, 10, 0, 0)

    function read(relative) {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl(relative), false);
        request.send();
        return request.responseText;
    }

    function sample() {
        return State.parseState(read("../dev/sample-state.json"));
    }

    function window(overrides) {
        return Object.assign({
            id: "session",
            label: "Session",
            usedPercent: 24,
            remainingPercent: 76,
            resetsAt: new Date(2026, 8, 23, 14, 30),
            tone: "good",
            pace: {
                severity: "healthy",
                evenPacePercent: 40,
                projectedPercent: 67,
                sparePercent: 33,
                runsOutAt: null
            },
            hidden: false
        }, overrides ?? {});
    }

    function test_density_steps() {
        verify(Density.isCompact({
            density: "compact"
        }));
        verify(!Density.isCompact({
            density: "normal"
        }));
        compare(Density.sectionGap(units, true), 8);
        compare(Density.sectionGap(units, false), 14);
        compare(Density.meterHeight(units, true), 4);
        compare(Density.meterHeight(units, false), 5);
        compare(Density.donutSize(units, true), 84);
        compare(Density.providerIcon(units, true), 14);
        compare(Density.cardGutter(units, true), 3);
        compare(Density.barRowTop(units, true), 5);
        compare(Density.barRowBottom(units, true), 6);
        compare(Density.trendHeight(units, true), 14);
        compare(Density.fontStep(true), 1);
        compare(Density.fontStep(false), 0);
    }

    function test_compact_trailing_countdown_and_exact() {
        compare(CompactRow.trailing("en", window(), now, "countdown", false, "24h"), "4h 30m");
        compare(CompactRow.trailing("en", window(), now, "exact", false, "24h"), "14:30");
        compare(CompactRow.trailing("en", window(), now, "exact", false, "12h"), "2:30 PM");
        const monday = window({
            resetsAt: new Date(2026, 8, 28, 9, 0)
        });
        compare(CompactRow.trailing("en", monday, now, "exact", false, "24h"), "Mon 09:00");
        compare(CompactRow.trailing("ru", monday, now, "exact", false, "24h"), "пн 09:00");
        compare(CompactRow.trailing("en", monday, now, "countdown", false, "24h"), "4d 23h");
    }

    function test_compact_trailing_edges() {
        compare(CompactRow.trailing("en", window({
            remainingPercent: null
        }), now, "countdown", false, "24h"), "No data");
        compare(CompactRow.trailing("en", window({
            resetsAt: null
        }), now, "countdown", false, "24h"), "Not started");
        compare(CompactRow.trailing("en", window({
            resetsAt: new Date(now.getTime() - 1000)
        }), now, "countdown", false, "24h"), "reset pending");
        compare(CompactRow.trailing("en", window({
            resetsAt: new Date(now.getTime() + 30000)
        }), now, "countdown", false, "24h"), "soon");
    }

    function test_compact_meter_tip_moves_forecast_into_tooltip() {
        const display = Settings.parseDisplay(null);
        compare(CompactRow.meterTip("en", window(), now, display), "Resets in 4h 30m\nAt this pace: ~33% left at reset");
    }

    function test_toggle_hints_follow_mode() {
        compare(CompactRow.valueHint("en", "left"), "Click to show used");
        compare(CompactRow.valueHint("en", "used"), "Click to show what is left");
        compare(CompactRow.resetHint("en", "countdown"), "Click to show the reset time");
        compare(CompactRow.resetHint("ru", "exact"), "Нажмите, чтобы показать обратный отсчёт");
    }

    function test_units() {
        const period = sample().spend.last30Days;
        compare(SpendUnits.unitOptions("en").map(unit => unit.value), ["cost", "tokens", "cost_per_mtok"]);
        compare(SpendUnits.unitTitle("en", "tokens"), "Total Tokens");
        compare(SpendUnits.unitTitle("en", "bogus"), "Total Spend");
        compare(SpendUnits.ringCaption("en", "cost"), "");
        compare(SpendUnits.ringCaption("en", "cost_per_mtok"), "blended");
        compare(SpendUnits.ringCaption("ru", "tokens"), "токенов");
        compare(SpendUnits.ringValue(period, "cost_per_mtok"), period.costPerMtokMicros === null ? "—" : SpendUnits.legendValue(period, "cost_per_mtok"));
        verify(SpendUnits.byTokens("tokens"));
        verify(SpendUnits.byTokens("cost_per_mtok"));
        verify(!SpendUnits.byTokens("cost"));
    }

    function test_ring_weights_follow_unit() {
        const providers = [
            {
                provider: "claude",
                costMicros: 3000000,
                totalTokens: 1000
            },
            {
                provider: "codex",
                costMicros: 1000000,
                totalTokens: 3000
            }
        ];
        compare(Math.round(Spend.slices(providers, 0, false)[0].sweep), 270);
        compare(Math.round(Spend.slices(providers, 0, false, true)[0].sweep), 90);
    }

    function period() {
        return {
            costMicros: 10000000,
            totalTokens: 1000,
            providers: [
                {
                    provider: "claude",
                    costMicros: 6000000,
                    totalTokens: 400,
                    models: [
                        {
                            model: "opus",
                            costMicros: 5000000,
                            totalTokens: 300,
                            partial: false
                        },
                        {
                            model: "haiku",
                            costMicros: 1000000,
                            totalTokens: 100,
                            partial: false
                        }
                    ],
                    modelsOther: null
                },
                {
                    provider: "codex",
                    costMicros: 4000000,
                    totalTokens: 600,
                    models: [
                        {
                            model: "gpt",
                            costMicros: 3500000,
                            totalTokens: 500,
                            partial: false
                        }
                    ],
                    modelsOther: {
                        count: 2,
                        costMicros: 500000,
                        totalTokens: 100,
                        partial: false
                    }
                }
            ],
            projects: [
                {
                    project: "~/code/headroom",
                    costMicros: 7000000,
                    totalTokens: 700,
                    partial: false,
                    sharePermille: 700,
                    costPerMtokMicros: 10000000,
                    providers: [
                        {
                            provider: "claude",
                            costMicros: 5000000,
                            totalTokens: 300
                        },
                        {
                            provider: "codex",
                            costMicros: 2000000,
                            totalTokens: 400
                        }
                    ]
                }
            ],
            projectsOther: {
                count: 3,
                costMicros: 3000000,
                totalTokens: 300,
                partial: false,
                sharePermille: 300,
                costPerMtokMicros: null
            }
        };
    }

    function test_model_breakdown_rows_are_ranked_and_add_up() {
        const result = SpendBreakdown.breakdown("en", period(), "models", "cost");
        compare(result.caption, "5 models");
        compare(result.rows.map(row => row.name), ["opus", "gpt", "haiku", "Other"]);
        compare(result.rows.map(row => row.value), ["$5.00", "$3.50", "$1.00", "$0.50"]);
        compare(result.rows.map(row => row.share), ["50.0%", "35.0%", "10.0%", "5.0%"]);
        compare(result.rows[3].detail, "2 models");
        compare(result.rows[0].segments, [
            {
                provider: "claude",
                fraction: 0.5
            }
        ]);
    }

    function test_model_breakdown_folds_after_top_five() {
        const many = period();
        many.providers[0].models = ["a", "b", "c", "d", "e", "f"].map((name, index) => ({
                    model: name,
                    costMicros: 1000000 - index,
                    totalTokens: 10,
                    partial: false
                }));
        const result = SpendBreakdown.breakdown("en", many, "models", "cost");
        compare(result.rows.length, 6);
        compare(result.rows[5].detail, "4 models");
        compare(result.caption, "9 models");
        compare(result.rows[5].segments.map(segment => segment.provider), ["codex", "claude"]);
    }

    function test_project_breakdown_uses_daemon_share_and_provider_segments() {
        const result = SpendBreakdown.breakdown("ru", period(), "projects", "cost");
        compare(result.caption, "4 проекта");
        compare(result.rows.map(row => row.name), ["~/code/headroom", "Другие"]);
        compare(result.rows.map(row => row.share), ["70,0%", "30,0%"]);
        verify(result.rows.every(row => row.folder));
        compare(result.rows[0].segments, [
            {
                provider: "claude",
                fraction: 0.5
            },
            {
                provider: "codex",
                fraction: 0.2
            }
        ]);
        compare(result.rows[1].segments, [
            {
                provider: "",
                fraction: 0.3
            }
        ]);
    }

    function test_model_breakdown_ranks_by_tokens_in_tokens_unit() {
        const result = SpendBreakdown.breakdown("en", period(), "models", "tokens");
        compare(result.rows.map(row => row.name), ["gpt", "opus", "haiku", "Other"]);
        compare(result.rows.map(row => row.value), ["500", "300", "100", "100"]);
        compare(result.rows.map(row => row.share), ["50.0%", "30.0%", "10.0%", "10.0%"]);
    }

    function test_model_breakdown_breaks_token_ties_by_name() {
        const tied = period();
        tied.providers[0].models[1].totalTokens = 300;
        const result = SpendBreakdown.breakdown("en", tied, "models", "tokens");
        compare(result.rows.map(row => row.name), ["gpt", "haiku", "opus", "Other"]);
    }

    function test_model_breakdown_shows_cost_per_mtok_from_payload() {
        const priced = period();
        priced.providers[0].models[0].costPerMtokMicros = 16666667;
        priced.providers[0].models[1].costPerMtokMicros = null;
        priced.providers[1].models[0].costPerMtokMicros = 7000000;
        const result = SpendBreakdown.breakdown("en", priced, "models", "cost_per_mtok");
        compare(result.rows.map(row => row.name), ["gpt", "opus", "haiku", "Other"]);
        compare(result.rows.map(row => row.value), ["$7.00", "$16.67", "—", "—"]);
        compare(result.caption, "5 models");
    }

    function test_project_breakdown_ranks_by_tokens_in_tokens_unit() {
        const busy = period();
        busy.projects.push({
            project: "~/code/site",
            costMicros: 1000000,
            totalTokens: 900,
            partial: false,
            sharePermille: 100,
            providers: []
        });
        compare(SpendBreakdown.breakdown("en", busy, "projects", "cost").rows.map(row => row.name), ["~/code/headroom", "~/code/site", "Other"]);
        compare(SpendBreakdown.breakdown("en", busy, "projects", "tokens").rows.map(row => row.name), ["~/code/site", "~/code/headroom", "Other"]);
    }

    function test_project_breakdown_shows_cost_per_mtok_from_payload() {
        const result = SpendBreakdown.breakdown("en", period(), "projects", "cost_per_mtok");
        compare(result.caption, "4 projects");
        compare(result.rows.map(row => row.value), ["$10.00", "—"]);
        compare(result.rows.map(row => row.share), ["70.0%", "30.0%"]);
        compare(SpendBreakdown.breakdown("ru", period(), "projects", "cost_per_mtok").caption, "4 проекта");
        const swapped = period();
        swapped.projects[0].costPerMtokMicros = null;
        swapped.projectsOther.costPerMtokMicros = 1250000;
        compare(SpendBreakdown.breakdown("en", swapped, "projects", "cost_per_mtok").rows.map(row => row.value), ["—", "$1.25"]);
    }

    function test_breakdown_by_tokens() {
        const result = SpendBreakdown.breakdown("en", period(), "projects", "tokens");
        compare(result.rows.map(row => row.value), ["700", "300"]);
        compare(result.rows[0].segments[1].fraction, 0.4);
    }

    function test_breakdown_hidden_without_projects() {
        const bare = period();
        bare.projects = null;
        compare(SpendBreakdown.breakdown("en", bare, "models", "cost"), null);
        bare.projects = [];
        verify(!SpendBreakdown.hasProjects(bare));
    }

    function test_popover_rows() {
        const claude = period().providers[1];
        claude.partial = false;
        const rows = Breakdown.popoverRows("en", claude);
        compare(rows.map(row => row.name), ["gpt", "Other"]);
        compare(rows.map(row => row.figures), ["87% · 500 tokens", "12% · 100 tokens"]);
        compare(rows[1].detail, "2 models");
        compare(rows[0].fraction, 0.875);
        compare(Breakdown.popoverNotes("en", claude), ["Models after the top five are folded into Other.", "Estimated from local logs and public pricing."]);
        compare(Breakdown.popoverRows("en", {
            models: []
        }), []);
    }

    function test_popover_title() {
        compare(Spend.popoverTitle("en", "30d", {
            providerName: "Claude"
        }), "Claude · 30 Days");
    }

    function test_incident_notice() {
        const status = {
            provider: "claude",
            indicator: "critical",
            tone: "critical",
            title: "Claude API unavailable",
            stage: "identified",
            startedAt: new Date(now.getTime() - 12 * 60000),
            url: "https://stspg.io/x"
        };
        compare(Incident.notice("en", status, now), {
            kind: "error",
            tone: "critical",
            headline: "Major outage",
            title: "Major outage · Claude API unavailable",
            detail: "Identified",
            started: "Started 12m ago",
            age: "12m",
            url: "https://stspg.io/x"
        });
        compare(Incident.notice("en", Object.assign({}, status, {
            indicator: "none",
            tone: "neutral"
        }), now), null);
        compare(Incident.notice("en", null, now), null);
        const bare = Incident.notice("en", Object.assign({}, status, {
            indicator: "minor",
            tone: "warning",
            title: null,
            stage: null,
            startedAt: null
        }), now);
        compare(bare.kind, "warning");
        compare(bare.title, "Degraded performance");
        compare(bare.started, "");
    }

    function test_quick_links() {
        const links = {
            status: "https://www.githubstatus.com",
            dashboard: "https://github.com/settings/copilot",
            usage: "https://github.com/settings/copilot"
        };
        compare(QuickLinks.headerEntries("en", links).map(entry => entry.kind), ["status", "dashboard"]);
        compare(QuickLinks.menuEntries("en", links)[0].tip, "Status page · githubstatus.com");
        compare(QuickLinks.menuEntries("en", links)[1].host, "github.com");
        compare(QuickLinks.menuEntries("en", links)[1].menuHost, "github.com");
        const long = QuickLinks.menuEntries("en", {
            status: "https://status.a-very-long-subdomain.of-some-provider-cloud.example.com",
            dashboard: null,
            usage: null
        })[0];
        compare(Array.from(long.menuHost).length, 40);
        verify(long.menuHost.startsWith("status.") && long.menuHost.endsWith("example.com") && long.menuHost.includes("…"));
        compare(QuickLinks.headerEntries("en", {
            status: null,
            dashboard: null,
            usage: "https://cursor.com/dashboard/usage"
        }).map(entry => entry.label), ["Usage page"]);
        compare(QuickLinks.headerEntries("en", undefined), []);
    }

    function test_collapse_helpers() {
        const card = (provider, name) => ({
                    kind: "account",
                    account: {
                        provider,
                        providerName: name
                    }
                });
        const folded = [card("copilot", "Copilot"), card("grok", "Grok"), card("copilot", "Copilot"), card("warp", "Warp"), card("cursor", "Cursor")];
        compare(Collapse.foldedCount("en", folded), "5 more");
        compare(Collapse.foldedNames(folded), "Copilot, Grok, Warp, Cursor");
        compare(Collapse.foldedProviders(folded, 3), ["copilot", "grok", "warp"]);
        compare(Collapse.foldedTitle("ru", folded), "Ещё 5 · Copilot, Grok, Warp, Cursor");
    }
}
