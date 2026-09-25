import QtQuick
import QtTest
import "../package/contents/ui/logic/Collapse.js" as Collapse
import "../package/contents/ui/logic/Combined.js" as Combined
import "../package/contents/ui/logic/Compat.js" as Compat
import "../package/contents/ui/logic/ProviderStatus.js" as ProviderStatus
import "../package/contents/ui/logic/Refresh.js" as Refresh
import "../package/contents/ui/logic/Spend.js" as Spend
import "../package/contents/ui/logic/State.js" as State

TestCase {
    name: "State06"

    function read(relative) {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl(relative), false);
        request.send();
        return request.responseText;
    }

    function rawSample() {
        return JSON.parse(read("../dev/sample-state.json"));
    }

    function parsed(raw) {
        return State.parseState(JSON.stringify(raw));
    }

    function without(raw, keys) {
        keys.forEach(key => delete raw[key]);
        return raw;
    }

    function test_sample_panel_items() {
        const state = parsed(rawSample());
        verify(state.supports06);
        verify(Compat.supports06(state));
        compare(state.panelTone, "critical");
        compare(state.panelItems, [
            {
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
                accountCount: 1,
                valuePercent: 62,
                evenPacePercent: 46,
                logo: "codex"
            }
        ]);
    }

    function test_panel_items_fall_back_to_headline_on_older_daemons() {
        const raw = without(rawSample(), ["panel_items", "panel_tone"]);
        raw.display.value_mode = "used";
        const state = parsed(raw);
        verify(!state.supports06);
        verify(!Compat.supports06(state));
        compare(state.panelTone, "good");
        compare(state.panelItems.length, 1);
        compare(state.panelItems[0].valuePercent, 38);
        compare(state.panelItems[0].evenPacePercent, 46);
        compare(state.panelItems[0].logo, "codex");
        raw.headline = null;
        compare(parsed(raw).panelItems, []);
        compare(parsed(raw).panelTone, null);
    }

    function test_malformed_panel_items_are_dropped() {
        const raw = rawSample();
        const item = raw.panel_items[0];
        raw.panel_items = [item, "bad", null, Object.assign({}, item, {
                remaining_percent: "62"
            }), Object.assign({}, item, {
                value_percent: null,
                logo: 7,
                tone: "purple",
                even_pace_percent: "x"
            }), item, item];
        raw.panel_tone = "loud";
        const state = parsed(raw);
        verify(state.supports06);
        compare(state.panelTone, null);
        compare(state.panelItems.length, 3);
        compare(state.panelItems[1].valuePercent, 62);
        compare(state.panelItems[1].logo, "codex");
        compare(state.panelItems[1].tone, "neutral");
        compare(state.panelItems[1].evenPacePercent, null);
        raw.panel_items = [];
        compare(parsed(raw).panelItems, []);
        verify(parsed(raw).supports06);
    }

    function test_account_refresh_and_collapsed() {
        const state = parsed(rawSample());
        compare(state.accounts[0].refresh, {
            mode: "idle",
            intervalSecs: 300,
            nextAt: new Date("2026-09-23T10:03:10Z"),
            reason: "schedule"
        });
        compare(state.accounts[0].source, "live");
        verify(Refresh.isLive(state.accounts[2]));
        verify(!Refresh.isLive(state.accounts[0]));
        compare(state.accounts[0].collapsed, false);
        const raw = rawSample();
        raw.accounts[0].refresh = {
            mode: "turbo",
            interval_secs: 5,
            next_at: "soon",
            reason: 3
        };
        raw.accounts[0].collapsed = "yes";
        raw.accounts[1].collapsed = true;
        delete raw.accounts[2].refresh;
        raw.accounts[3].refresh = "live";
        const edited = parsed(raw);
        compare(edited.accounts[0].refresh, {
            mode: "idle",
            intervalSecs: 60,
            nextAt: null,
            reason: "schedule"
        });
        compare(edited.accounts[0].collapsed, false);
        compare(edited.accounts[1].collapsed, true);
        compare(edited.accounts[2].refresh, null);
        compare(edited.accounts[3].refresh, null);
        verify(!Refresh.isLive(edited.accounts[2]));
    }

    function test_collapsed_cards_fold_in_order() {
        const raw = rawSample();
        raw.accounts[5].collapsed = true;
        raw.accounts[6].collapsed = true;
        const state = parsed(raw);
        const cards = Combined.cards(state, State.visibleAccounts(state));
        const parts = Collapse.partition(cards);
        compare(parts.folded.map(card => card.id), ["zai:7b8c9d0e1f2a", "openrouter:4d5e6f7a8b9c"]);
        compare(parts.shown.length, cards.length - 2);
        compare(Collapse.foldedTitle("en", parts.folded), "2 more · Z.ai, OpenRouter");
        compare(Collapse.foldedTitle("ru", parts.folded), "Ещё 2 · Z.ai, OpenRouter");
        compare(Collapse.foldedTitle("en", []), "0 more");
    }

    function test_combined_collapsed() {
        const raw = rawSample();
        raw.display.combine_accounts = true;
        raw.combined = [
            {
                provider: "codex",
                provider_name: "Codex",
                account_ids: ["codex:1a2b3c4d5e6f", "codex:9f8e7d6c5b4a"],
                accounts: [],
                windows: [],
                collapsed: true
            }
        ];
        const state = parsed(raw);
        compare(state.combined[0].collapsed, true);
        const cards = Combined.cards(state, State.visibleAccounts(state));
        compare(Collapse.partition(cards).folded.map(card => card.kind), ["combined"]);
        delete raw.combined[0].collapsed;
        compare(parsed(raw).combined[0].collapsed, false);
    }

    function test_provider_status() {
        const state = parsed(rawSample());
        compare(state.providerStatus.length, 2);
        const claude = ProviderStatus.forProvider(state.providerStatus, "claude");
        compare(claude, {
            provider: "claude",
            indicator: "minor",
            tone: "warning",
            title: "Elevated errors on Claude Code",
            stage: "identified",
            startedAt: new Date("2026-09-23T09:12:00Z"),
            url: "https://stspg.io/abc123"
        });
        verify(ProviderStatus.hasIssue(claude));
        verify(!ProviderStatus.hasIssue(ProviderStatus.forProvider(state.providerStatus, "codex")));
        verify(!ProviderStatus.hasIssue(ProviderStatus.forProvider(state.providerStatus, "zai")));
        compare(ProviderStatus.stageLabel("ru", claude.stage), "Причина найдена");
        compare(ProviderStatus.stageLabel("en", "escalated"), "escalated");
        compare(ProviderStatus.indicatorLabel("en", "major"), "Partial outage");
        compare(ProviderStatus.indicatorLabel("en", "none"), "");
    }

    function test_provider_status_tolerates_bad_entries() {
        const raw = rawSample();
        raw.provider_status = [
            {
                provider: "cursor",
                indicator: "critical",
                url: "http://insecure.example"
            },
            {
                provider: "warp",
                indicator: "sideways"
            },
            {
                indicator: "minor"
            },
            "claude"];
        const statuses = parsed(raw).providerStatus;
        compare(statuses.length, 1);
        compare(statuses[0].tone, "critical");
        compare(statuses[0].url, null);
        compare(statuses[0].title, null);
        delete raw.provider_status;
        compare(parsed(raw).providerStatus, []);
    }

    function test_spend_additions() {
        const spend = parsed(rawSample()).spend;
        verify(spend.last7Days !== null);
        compare(spend.today.costPerMtokMicros, rawSample().spend.today.cost_per_mtok_usd_micros);
        compare(spend.today.providers[0].costPerMtokMicros, rawSample().spend.today.by_provider[0].cost_per_mtok_usd_micros);
        verify(Number.isInteger(spend.today.providers[0].costPerMtokMicros));
        compare(spend.today.providers[0].models[0].costPerMtokMicros, rawSample().spend.today.by_provider[0].models[0].cost_per_mtok_usd_micros);
        const project = spend.today.projects[0];
        compare(project.project, "~/code/headroom");
        compare(project.sharePermille, 460);
        compare(project.costPerMtokMicros, rawSample().spend.today.projects[0].cost_per_mtok_usd_micros);
        verify(Number.isInteger(project.costPerMtokMicros));
        compare(project.providers[0], {
            provider: "codex",
            providerName: "Codex",
            costMicros: 6610200,
            totalTokens: 2213520
        });
        compare(spend.today.projectsOther.count, 3);
        compare(spend.today.projectsOther.costPerMtokMicros, rawSample().spend.today.projects_other.cost_per_mtok_usd_micros);
        verify(Spend.hasProjects(spend.today));
        compare(Spend.periodOptions("en", spend).map(option => option.value), ["today", "yesterday", "7d", "30d"]);
        compare(Spend.periodTotals(spend, "7d"), spend.last7Days);
        compare(Spend.periodTitle("ru", "7d"), "7 дней");
    }

    function test_spend_without_06_fields() {
        const raw = rawSample();
        delete raw.spend.last_7_days;
        delete raw.spend.today.projects;
        delete raw.spend.today.projects_other;
        delete raw.spend.today.cost_per_mtok_usd_micros;
        raw.spend.yesterday.projects = [
            {
                project: null,
                cost_usd_micros: "1",
                share_permille: 1000
            }
        ];
        raw.spend.yesterday.projects_other = {
            count: 0
        };
        const spend = parsed(raw).spend;
        compare(spend.last7Days, null);
        compare(spend.today.projects, null);
        compare(spend.today.projectsOther, null);
        compare(spend.today.costPerMtokMicros, null);
        verify(!Spend.hasProjects(spend.today));
        compare(spend.yesterday.projects[0].project, null);
        compare(spend.yesterday.projects[0].costMicros, 0);
        compare(spend.yesterday.projects[0].providers, []);
        compare(spend.yesterday.projects[0].costPerMtokMicros, null);
        compare(spend.yesterday.projectsOther, null);
        compare(Spend.periodOptions("en", spend).map(option => option.value), ["today", "yesterday", "30d"]);
        compare(Spend.periodOptions("en").map(option => option.value), ["today", "yesterday", "30d"]);
        compare(Spend.periodTotals(spend, "7d"), spend.last30Days);
    }

    function test_combined_sample_carries_06_keys() {
        const state = State.parseState(read("../dev/sample-combined.json"));
        verify(state.supports06);
        compare(state.panelItems[0].combined, true);
        compare(state.panelItems[0].accountCount, 2);
        compare(state.combined[0].collapsed, false);
        verify(state.spend.last7Days !== null);
    }
}
