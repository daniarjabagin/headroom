import QtQuick
import QtTest
import "../package/contents/ui/logic/Format.js" as Format
import "../package/contents/ui/logic/Quota.js" as Quota
import "../package/contents/ui/logic/Spend.js" as Spend
import "../package/contents/ui/logic/Trend.js" as Trend

TestCase {
    readonly property date now: new Date("2026-09-23T10:00:00Z")

    function window(overrides) {
        return Object.assign({
            label: "Session",
            remainingPercent: 62,
            resetsAt: new Date("2026-09-23T12:41:00Z"),
            tone: "good",
            pace: {
                severity: "healthy",
                evenPacePercent: 46,
                projectedPercent: 82,
                sparePercent: 18,
                runsOutAt: null
            }
        }, overrides);
    }

    function test_percent() {
        compare(Format.percentLeft(61.6), "62% left");
        compare(Format.percentLeft(null), "—");
        compare(Format.panelPercent(-0.4), "0%");
    }

    function test_countdowns() {
        compare(Format.resetText(new Date("2026-09-23T12:41:00Z"), now), "Resets in 2h 41m");
        compare(Format.resetText(new Date("2026-09-27T16:30:00Z"), now), "Resets in 4d 6h");
        compare(Format.resetText(new Date("2026-09-23T10:00:30Z"), now), "Resets soon");
        compare(Format.resetText(null, now), "Not started");
        compare(Format.limitText(new Date("2026-09-24T19:00:00Z"), now), "Limit in 1d 9h");
        compare(Format.limitText(null, now), "Limit soon");
        compare(Format.nextUpdateText(new Date("2026-09-23T10:03:10Z"), now), "Next update in 3m");
        compare(Format.nextUpdateText(new Date("2026-09-23T10:00:40Z"), now), "Next update in <1m");
        compare(Format.agoText(new Date("2026-09-23T07:00:00Z"), now), "3h 0m ago");
    }

    function test_pace_copy() {
        compare(Format.spareText(4.2), "~4% spare");
        compare(Format.leftAtResetText(18), "~18% left at reset");
    }

    function test_tokens() {
        compare(Format.compactTokens(1200), "1.2K");
        compare(Format.compactTokens(35812904), "35.8M");
        compare(Format.compactTokens(1500000000), "1.5B");
        compare(Format.compactTokens(999), "999");
        compare(Format.compactTokens(3000000), "3M");
        compare(Format.exactTokens(1203448), "1,203,448");
    }

    function test_money() {
        compare(Format.usd(14370000), "$14.37");
        compare(Format.usd(2064000000), "$2.06K");
        compare(Format.usd(4050000), "$4.05");
        compare(Format.exactUsd(1234567890), "$1,234.57");
        compare(Format.ringUsd(463120000), "$463");
        compare(Format.ringUsd(18420000), "$18.42");
        compare(Format.spendLine({
            costMicros: 4080000,
            totalTokens: 1203448
        }), "$4.08 · 1.2M tokens");
        compare(Format.spendLine({
            costMicros: 0,
            totalTokens: 0
        }), "No data");
        compare(Format.balanceValue({
            kind: "count",
            value: 1200,
            unit: "requests"
        }), "1,200 requests");
    }

    function test_quota_rows() {
        compare(Quota.tickPosition(window({}), false), null);
        compare(Quota.tickPosition(window({}), true), 0.54);
        compare(Quota.tickPosition(window({
            tone: "warning"
        }), false), 0.54);
        compare(Quota.paceNote(window({}), now, false), null);
        compare(Quota.paceNote(window({}), now, true).text, "~18% left at reset");
        const spent = window({
            remainingPercent: 0,
            pace: {
                severity: "spent",
                evenPacePercent: 20,
                sparePercent: null,
                runsOutAt: null
            }
        });
        compare(Quota.paceNote(spent, now, false), {
            flame: true,
            text: "Limit reached"
        });
        compare(Quota.fillFraction(spent), 0);
        compare(Quota.trailingText(window({
            remainingPercent: null
        }), now), "No data");
        compare(Quota.meterTone(window({
            remainingPercent: null
        })), "neutral");
    }

    function test_spend_slices() {
        const slices = Spend.slices([
            {
                provider: "codex",
                costMicros: 900
            },
            {
                provider: "claude",
                costMicros: 100
            }
        ], 2);
        compare(slices.length, 2);
        compare(slices[0].start, -89);
        compare(slices[0].sweep, 322);
        compare(slices[1].color, "#DE7356");
        compare(Spend.slices([
            {
                provider: "codex",
                costMicros: 5
            }
        ], 2)[0].sweep, 360);
        compare(Spend.bodyKind({
            providers: []
        }), "empty");
    }

    function test_trend() {
        const days = Trend.lastDays([
            {
                date: "2026-09-23",
                totalTokens: 10
            }
        ]);
        compare(days.length, 30);
        compare(days[29].totalTokens, 10);
        compare(Trend.barShare(1, 100), 0.18);
        compare(Trend.barShare(0, 100), 0);
        compare(Trend.peakDescription(days), "Peak 10 tokens on 2026-09-23");
    }
}
