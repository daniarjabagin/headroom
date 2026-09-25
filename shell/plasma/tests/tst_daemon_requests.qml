import QtQuick
import QtTest
import org.kde.plasma.workspace.dbus as DBus
import headroom.preview
import "../package/contents/ui"
import "../package/contents/ui/logic/Diagnostics.js" as Diagnostics
import "../package/contents/ui/logic/SpendQuery.js" as SpendQuery

TestCase {
    id: suite

    readonly property int settleMs: 2000
    property DaemonClient client: null
    property var answers: []

    name: "DaemonRequests"

    function members() {
        return DBus.SessionBus.messages.map(message => message.member);
    }

    function sent(member) {
        return DBus.SessionBus.messages.filter(message => message.member === member);
    }

    function answered(error, value) {
        answers = answers.concat([
            {
                error,
                value
            }
        ]);
    }

    function start(statePatch) {
        PreviewConfig.appliedSettings = null;
        PreviewConfig.failingMembers = [];
        PreviewConfig.statePatch = statePatch;
        answers = [];
        client = clientComponent.createObject(suite) as DaemonClient;
        tryVerify(() => client.settings !== null && client.view.kind === "ready", settleMs);
        DBus.SessionBus.messages = [];
    }

    function cleanup() {
        if (client !== null)
            client.destroy();
        client = null;
        PreviewConfig.statePatch = {};
        PreviewConfig.failingMembers = [];
        PreviewConfig.appliedSettings = null;
    }

    function test_get_spend_sends_the_query_and_parses_rows() {
        start({});
        verify(client.supports06);
        client.getSpend(SpendQuery.periodQuery("7d", "provider", ""), answered);
        compare(members(), ["GetSpend"]);
        compare(sent("GetSpend")[0].signature, "(s)");
        compare(JSON.parse(sent("GetSpend")[0].arguments[0]), {
            period: "7d",
            by: "provider"
        });
        tryVerify(() => suite.answers.length === 1, settleMs);
        compare(answers[0].error, null);
        const result = answers[0].value;
        compare(result.by, "provider");
        verify(result.rows.length > 0);
        compare(result.total.key, null);
        compare(result.total.sharePermille, 1000);
        compare(result.rows.reduce((sum, row) => sum + row.costMicros, 0), result.total.costMicros);
    }

    function test_get_diagnostics_returns_the_report() {
        start({});
        client.getDiagnostics(answered);
        compare(members(), ["GetDiagnostics"]);
        compare(sent("GetDiagnostics")[0].arguments, []);
        tryVerify(() => suite.answers.length === 1, settleMs);
        compare(answers[0].error, null);
        compare(answers[0].value.logFile, "~/.local/state/headroom/headroom.log");
        verify(answers[0].value.text.startsWith("Headroom 0.6.0"));
    }

    function test_request_failures_reach_the_caller() {
        start({});
        PreviewConfig.failingMembers = ["GetDiagnostics"];
        client.getDiagnostics(answered);
        tryVerify(() => suite.answers.length === 1, settleMs);
        compare(answers[0].error, "GetDiagnostics failed");
        compare(answers[0].value, null);
    }

    function test_reset_settings_reloads_settings() {
        start({});
        client.updateSettings({
            display: {
                density: "compact"
            }
        });
        tryVerify(() => !client.patchQueue.busy, settleMs);
        tryCompare(client.settings.display, "density", "compact", settleMs);
        DBus.SessionBus.messages = [];
        client.resetSettings();
        compare(members(), ["ResetSettings"]);
        compare(sent("ResetSettings")[0].signature, "");
        tryVerify(() => members().includes("GetSettings"), settleMs);
        tryCompare(client.settings.display, "density", "normal", settleMs);
        verify(client.settings.onboarding.completed);
    }

    function test_older_daemons_get_no_new_keys_or_methods() {
        start({
            panel_items: undefined
        });
        verify(!client.supports06);
        client.updateSettings({
            display: {
                density: "compact",
                theme: "dark"
            }
        });
        client.updateSettings({
            logging: {
                level: "debug"
            }
        });
        client.patchDisplay({
            timeFormat: "12h"
        });
        client.resetSettings();
        client.getSpend(SpendQuery.periodQuery("today", "model", ""), answered);
        client.getDiagnostics(answered);
        tryVerify(() => !client.patchQueue.busy, settleMs);
        compare(sent("UpdateSettings").map(message => JSON.parse(message.arguments[0])), [
            {
                display: {
                    theme: "dark"
                }
            }
        ]);
        verify(!members().includes("ResetSettings"));
        verify(!members().includes("GetSpend"));
        verify(!members().includes("GetDiagnostics"));
        compare(answers.map(answer => answer.error), ["Needs a newer Headroom service", "Needs a newer Headroom service"]);
        compare(client.view.state.display.timeFormat, "auto");
    }

    function test_parse_spend_results() {
        compare(SpendQuery.parseResult("{"), null);
        compare(SpendQuery.parseResult("[]"), null);
        compare(SpendQuery.parseResult(JSON.stringify({
            since: "2026-09-17",
            until: "2026-09-23",
            by: "model",
            rows: []
        })), null);
        const result = SpendQuery.parseResult(JSON.stringify({
            since: "2026-09-17",
            until: "2026-09-23",
            by: "model",
            rows: [
                {
                    key: "claude-opus-4-5",
                    provider: "claude",
                    tokens: {
                        input: 1200000,
                        cache_read: 180000000,
                        cache_write: 9000000,
                        output: 2100000,
                        reasoning: 0,
                        total: 192300000
                    },
                    cost_usd_micros: 151200000,
                    partial: false,
                    unpriced_tokens: 0,
                    cost_per_mtok_usd_micros: 786271,
                    share_permille: 767
                },
                "junk"],
            total: {
                key: null,
                provider: null,
                cost_usd_micros: 151200000,
                cost_per_mtok_usd_micros: null
            }
        }));
        compare(result.rows.length, 1);
        compare(result.rows[0].tokens.cacheRead, 180000000);
        compare(result.rows[0].costPerMtokMicros, 786271);
        compare(result.rows[0].sharePermille, 767);
        compare(result.total.tokens.total, 0);
        compare(result.total.costPerMtokMicros, null);
    }

    function test_build_spend_queries() {
        compare(SpendQuery.periodQuery("30d", "project", "claude"), {
            period: "30d",
            by: "project",
            provider: "claude"
        });
        compare(SpendQuery.periodQuery("week", "project", ""), null);
        compare(SpendQuery.periodQuery("today", "session", ""), null);
        compare(SpendQuery.rangeQuery("2026-09-01", null, "day", null), {
            since: "2026-09-01",
            by: "day"
        });
        compare(SpendQuery.rangeQuery("2026-09-01", "2026-09-10", "day", "codex"), {
            since: "2026-09-01",
            until: "2026-09-10",
            by: "day",
            provider: "codex"
        });
        compare(SpendQuery.rangeQuery("2026-09-10", "2026-09-01", "day", null), null);
        compare(SpendQuery.rangeQuery("09/01/2026", null, "day", null), null);
    }

    function test_parse_diagnostics() {
        compare(Diagnostics.parseDiagnostics("nope"), null);
        compare(Diagnostics.parseDiagnostics(JSON.stringify({
            app_version: "0.6.0"
        })), null);
        const report = Diagnostics.parseDiagnostics(JSON.stringify({
            app_version: "0.6.0",
            os: null,
            log_level: "loud",
            log_level_source: "env",
            log_file: null,
            text: "Headroom 0.6.0\n"
        }));
        compare(report.os, null);
        compare(report.logLevel, null);
        compare(report.logLevelSource, "env");
        compare(report.logFile, null);
        compare(Diagnostics.expandedPath("~/.local/state/headroom/headroom.log", "/home/ada/"), "/home/ada/.local/state/headroom/headroom.log");
        compare(Diagnostics.expandedPath("/var/log/headroom.log", "/home/ada"), "/var/log/headroom.log");
        compare(Diagnostics.folderOf("/home/ada/.local/state/headroom/headroom.log"), "/home/ada/.local/state/headroom");
        compare(Diagnostics.folderOf("/headroom.log"), "/");
    }

    Component {
        id: clientComponent

        DaemonClient {
            trackSettings: true
        }
    }
}
