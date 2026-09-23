import QtQuick
import QtTest
import "../package/contents/ui/logic/Breakdown.js" as Breakdown
import "../package/contents/ui/logic/Format.js" as Format
import "../package/contents/ui/logic/Quota.js" as Quota
import "../package/contents/ui/logic/Sector.js" as Sector
import "../package/contents/ui/logic/Spend.js" as Spend
import "../package/contents/ui/logic/Trend.js" as Trend

TestCase {
    readonly property date now: new Date("2026-09-23T10:00:00Z")
    readonly property var countdown: ({
            valueMode: "left",
            resetFormat: "countdown",
            showForecast: true
        })

    function window(overrides) {
        return Object.assign({
            id: "session",
            label: "Session",
            usedPercent: 38,
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

    function localDate(day, hours, minutes) {
        return new Date(2026, 8, day, hours, minutes);
    }

    function test_percent_data() {
        return [
            {
                lang: "en",
                left: "62% left",
                used: "38% used"
            },
            {
                lang: "ru",
                left: "Осталось 62%",
                used: "Использовано 38%"
            }
        ];
    }

    function test_percent(row) {
        compare(Format.percentLeft(row.lang, 61.6), row.left);
        compare(Format.percentUsed(row.lang, 38.2), row.used);
        compare(Format.readingFor(row.lang, 38, "used"), row.used);
        compare(Format.percentLeft(row.lang, null), "—");
        compare(Format.panelPercent(-0.4), "0%");
    }

    function test_durations_data() {
        const minute = 60 * 1000;
        return [
            {
                tag: "hours",
                ms: 161 * minute,
                live: false,
                en: "2h 41m",
                ru: "2 ч 41 мин"
            },
            {
                tag: "days",
                ms: (4 * 24 * 60 + 5 * 60) * minute,
                live: false,
                en: "4d 5h",
                ru: "4 д 5 ч"
            },
            {
                tag: "live minutes",
                ms: 12 * minute + 7000,
                live: true,
                en: "12m 07s",
                ru: "12 мин 07 с"
            },
            {
                tag: "live seconds",
                ms: 42000,
                live: true,
                en: "42s",
                ru: "42 с"
            },
            {
                tag: "coarse minutes",
                ms: 12 * minute + 7000,
                live: false,
                en: "12m",
                ru: "12 мин"
            }
        ];
    }

    function test_durations(row) {
        compare(Format.duration("en", row.ms, row.live), row.en);
        compare(Format.duration("ru", row.ms, row.live), row.ru);
    }

    function test_countdowns() {
        compare(Format.resetText("en", new Date("2026-09-23T12:41:00Z"), now), "Resets in 2h 41m");
        compare(Format.resetText("ru", new Date("2026-09-23T12:41:00Z"), now), "Сброс через 2 ч 41 мин");
        compare(Format.resetText("en", new Date("2026-09-23T10:00:30Z"), now), "Resets soon");
        compare(Format.resetText("en", new Date("2026-09-23T10:12:07Z"), now, "countdown", true), "Resets in 12m 07s");
        compare(Format.resetText("ru", null, now), "Не начато");
        compare(Format.limitText("en", new Date("2026-09-24T19:00:00Z"), now), "Limit in 1d 9h");
        compare(Format.limitText("ru", new Date("2026-09-24T19:00:00Z"), now), "Лимит через 1 д 9 ч");
        compare(Format.limitText("en", null, now), "Limit soon");
        compare(Format.nextUpdateText("en", new Date("2026-09-23T10:03:10Z"), now), "Next update in 3m");
        compare(Format.nextUpdateText("ru", new Date("2026-09-23T10:03:10Z"), now), "Обновление через 3 мин");
        compare(Format.nextUpdateText("en", new Date("2026-09-23T10:00:40Z"), now), "Next update in <1m");
        compare(Format.agoText("en", new Date("2026-09-23T07:00:00Z"), now), "3h 0m ago");
    }

    function test_exact_moments() {
        const base = localDate(23, 10, 0);
        compare(Format.resetText("en", localDate(23, 17, 5), base, "exact"), "Resets today at 17:05");
        compare(Format.resetText("ru", localDate(23, 17, 5), base, "exact"), "Сброс сегодня в 17:05");
        compare(Format.resetText("ru", localDate(24, 9, 30), base, "exact"), "Сброс завтра в 09:30");
        compare(Format.resetText("en", localDate(24, 9, 30), base, "exact"), "Resets tomorrow at 09:30");
        compare(Format.resetText("en", localDate(26, 8, 0), base, "exact"), "Resets Sat at 08:00");
        compare(Format.resetText("ru", localDate(26, 8, 0), base, "exact"), "Сброс сб в 08:00");
        compare(Format.resetText("ru", new Date(2026, 9, 12, 8, 0), base, "exact"), "Сброс 12 окт в 08:00");
        compare(Format.resetText("en", localDate(23, 9, 59), base, "exact"), "Reset pending");
        compare(Format.resetText("ru", localDate(22, 23, 0), base, "exact"), "Ожидается сброс");
        compare(Format.resetText("en", base, base, "exact"), "Reset pending");
    }

    function test_forecast() {
        compare(Format.spareText("ru", 4.2), "~4% запаса");
        compare(Format.forecastText("en", window({}), now, countdown), "At this pace: ~18% left at reset");
        compare(Format.forecastText("ru", window({}), now, countdown), "При текущем темпе к сбросу останется ~18%");
        compare(Format.forecastText("en", window({}), now, Object.assign({}, countdown, {
            valueMode: "used"
        })), "At this pace: ~82% used at reset");
        const runningOut = window({
            resetsAt: new Date("2026-09-23T12:28:00Z"),
            pace: {
                severity: "running_out",
                evenPacePercent: 50,
                projectedPercent: 120,
                sparePercent: null,
                runsOutAt: new Date("2026-09-23T10:45:00Z")
            }
        });
        compare(Format.forecastText("ru", runningOut, now, countdown), "При текущем темпе: закончится через 45 мин · сброс через 2 ч 28 мин");
        compare(Format.forecastText("en", runningOut, now, countdown), "At this pace: runs out in 45m · resets in 2h 28m");
        compare(Format.forecastText("en", window({
            remainingPercent: null
        }), now, countdown), null);
    }

    function test_window_labels() {
        compare(Format.windowLabel("ru", window({})), "Сессия");
        compare(Format.windowLabel("ru", window({
            id: "model:opus",
            label: "Opus"
        })), "Opus");
        compare(Format.shortWindowLabel("en", "weekly", "Weekly"), "W");
        compare(Format.shortWindowLabel("ru", "session", "Session"), "С");
        compare(Format.shortWindowLabel("en", "model:opus", "Opus"), "Opus");
    }

    function test_tokens() {
        compare(Format.compactTokens(1200), "1.2K");
        compare(Format.compactTokens(35812904), "35.8M");
        compare(Format.compactTokens(1500000000), "1.5B");
        compare(Format.compactTokens(999), "999");
        compare(Format.compactTokens(3000000), "3M");
        compare(Format.exactTokens(1203448), "1,203,448");
        compare(Format.tokenCount("ru", 1203448), "1,203,448 токенов");
        compare(Format.tokenCount("ru", 21), "21 токен");
        compare(Format.tokenCount("ru", 3), "3 токена");
        compare(Format.tokenCount("en", 1), "1 token");
    }

    function test_money() {
        compare(Format.usd(14370000), "$14.37");
        compare(Format.usd(2064000000), "$2.06K");
        compare(Format.exactUsd(1234567890), "$1,234.57");
        compare(Format.ringUsd(463120000), "$463");
        compare(Format.ringUsd(18420000), "$18.42");
        compare(Format.spendLine("en", {
            costMicros: 4080000,
            totalTokens: 1203448
        }), "$4.08 · 1.2M tokens");
        compare(Format.spendLine("ru", {
            costMicros: 0,
            totalTokens: 0
        }), "Нет данных");
        compare(Format.balanceValue("en", {
            kind: "count",
            value: 1200,
            unit: "requests"
        }), "1,200 requests");
    }

    function test_quota_rows() {
        const display = {
            valueMode: "left",
            showForecast: false
        };
        compare(Quota.tickPosition(window({}), display), null);
        compare(Quota.tickPosition(window({}), Object.assign({}, display, {
            showForecast: true
        })), 0.54);
        compare(Quota.tickPosition(window({
            tone: "warning"
        }), Object.assign({}, display, {
            valueMode: "used"
        })), 0.46);
        compare(Quota.fillFraction(window({}), "used"), 0.38);
        compare(Quota.paceNote("en", window({}), now, false), null);
        const spent = window({
            remainingPercent: 0,
            usedPercent: 100,
            pace: {
                severity: "spent",
                evenPacePercent: 20,
                sparePercent: null,
                runsOutAt: null
            }
        });
        compare(Quota.paceNote("ru", spent, now, true), {
            flame: true,
            text: "Лимит исчерпан"
        });
        compare(Quota.fillFraction(spent, "left"), 0);
        compare(Quota.trailingText("en", window({
            remainingPercent: null
        }), now, "countdown", false), "No data");
        compare(Quota.meterTone(window({
            remainingPercent: null
        })), "neutral");
        compare(Quota.forecast("en", window({}), now, display), null);
    }

    function spendOf(costs) {
        return costs.map((costMicros, index) => ({
                    provider: ["codex", "claude", "cursor", "grok"][index],
                    costMicros
                }));
    }

    readonly property var ringGeometry: Sector.geometry(104, 0.618, 2)

    function assertContiguous(slices) {
        let start = -90;
        slices.forEach((slice, index) => {
            fuzzyCompare(slice.start, start, 1e-9, `start of slice ${index}`);
            start += slice.sweep;
        });
        fuzzyCompare(start, 270, 1e-9);
    }

    function offsetFrom(edgeDegrees, point) {
        const angle = Sector.radians(edgeDegrees);
        const dx = point.x - ringGeometry.center;
        const dy = point.y - ringGeometry.center;
        return dy * Math.cos(angle) - dx * Math.sin(angle);
    }

    function test_spend_slices() {
        const slices = Spend.slices(spendOf([900, 100]), 10);
        compare(slices[0].start, -90);
        fuzzyCompare(slices[0].sweep, 324, 1e-9);
        fuzzyCompare(slices[1].start, 234, 1e-9);
        fuzzyCompare(slices[1].sweep, 36, 1e-9);
        compare(slices[1].color, "#D97757");
        compare(Spend.slices(spendOf([900, 100]), 10, true)[0].color, "#19C37D");
        assertContiguous(slices);
        compare(Spend.revealed(slices[0], 0), 0);
        fuzzyCompare(Spend.revealed(slices[0], 1), 324, 1e-9);
        compare(Spend.revealed(slices[1], 0.5), 0);
    }

    function test_spend_slices_keep_tiny_segments_visible() {
        const minimum = Sector.minSweep(ringGeometry);
        const slices = Spend.slices(spendOf([1000000, 1, 0]), minimum);
        compare(slices[1].sweep, slices[2].sweep);
        verify(slices[2].sweep >= minimum);
        assertContiguous(slices);
        verify(Sector.sectorPath(ringGeometry, slices[2].start, slices[2].sweep) !== "");
        verify(Spend.revealed(slices[2], 1) > 0);
        const fractions = Spend.visibleFractions([1000000, 1, 0], 0.1);
        fuzzyCompare(fractions.reduce((sum, value) => sum + value, 0), 1, 1e-9);
        compare(fractions.slice(1).map(value => Math.round(value * 1000)), [100, 100]);
    }

    function test_spend_slices_single_provider_is_full_ring() {
        compare(Spend.slices(spendOf([5]), 10), [
            {
                start: -90,
                sweep: 360,
                color: "#10A37F"
            }
        ]);
        compare(Spend.visibleFractions([0, 0], 0.1), [0.5, 0.5]);
    }

    function test_spend_slices_many_providers_stay_contiguous() {
        assertContiguous(Spend.slices(spendOf([50, 30, 2, 18]), Sector.minSweep(ringGeometry)));
    }

    function test_sector_geometry() {
        compare(ringGeometry.center, 52);
        compare(ringGeometry.outer, 52);
        fuzzyCompare(ringGeometry.inner, 32.136, 1e-9);
        fuzzyCompare(ringGeometry.corner, (52 - 32.136) * 0.15, 1e-9);
    }

    function test_sector_gaps_have_even_width() {
        const boundary = 30;
        const before = Sector.corners(ringGeometry, -60, 90);
        const after = Sector.corners(ringGeometry, boundary, 120);
        [before.outerEnd.onEdge, before.innerEnd.onEdge].forEach(point => fuzzyCompare(offsetFrom(boundary, point), -1, 1e-9));
        [after.outerStart.onEdge, after.innerStart.onEdge].forEach(point => fuzzyCompare(offsetFrom(boundary, point), 1, 1e-9));
    }

    function test_sector_corner_radius_is_clamped() {
        fuzzyCompare(Sector.cornerRadius(ringGeometry, 90), ringGeometry.corner, 1e-9);
        const tiny = Sector.minSweep(ringGeometry) / 2;
        const clamped = Sector.cornerRadius(ringGeometry, tiny);
        verify(clamped < ringGeometry.corner);
        fuzzyCompare(clamped, Sector.innerLength(ringGeometry, tiny) / 2, 1e-9);
        const k = Sector.corners(ringGeometry, 0, tiny);
        verify(k.innerStart.centerAngle <= k.innerEnd.centerAngle + 1e-9);
        compare(Sector.cornerRadius(ringGeometry, 1), 0);
    }

    function test_sector_path_shape() {
        const path = Sector.sectorPath(ringGeometry, -90, 200);
        verify(path.startsWith("M "));
        verify(path.endsWith(" Z"));
        compare(path.split(" L ").length, 3);
        compare(path.split(" A ").length, 7);
        verify(path.includes("A 52.000 52.000 0 1 1"));
        verify(Sector.sectorPath(ringGeometry, -90, 90).includes("A 52.000 52.000 0 0 1"));
        compare(Sector.sectorPath(ringGeometry, -90, 1), "");
    }

    function test_sector_single_ring_has_no_gap() {
        const ring = Sector.slicePath(Sector.geometry(104, 0.618, 0), -90, 360);
        compare(ring.split("M ").length, 3);
        verify(!ring.includes(" L "));
        compare(Sector.slicePath(ringGeometry, -90, 180), Sector.sectorPath(ringGeometry, -90, 180));
    }

    function test_spend_titles() {
        compare(Spend.periodTitle("ru", "last30Days"), "30 дней");
        compare(Spend.breakdownTitle("en", "today", {
            provider: "claude",
            providerName: "Claude"
        }), "Today · Claude");
        compare(Spend.bodyKind({
            providers: []
        }), "empty");
    }

    function test_breakdown() {
        const models = [1, 2, 3, 4, 5].map(index => ({
                    model: `m${index}`,
                    totalTokens: 1000 * index,
                    costMicros: 10000 * index,
                    partial: false
                }));
        const other = {
            count: 2,
            totalTokens: 13000,
            costMicros: 60000,
            partial: true
        };
        const breakdown = Breakdown.modelBreakdown("en", models, other);
        compare(breakdown.rows.length, 6);
        compare(breakdown.rows[5], {
            name: "Other (2)",
            tokens: "13K",
            cost: "$0.06",
            partial: true
        });
        compare(breakdown.partial, true);
        compare(Breakdown.modelBreakdown("ru", models, other).rows[5].name, "Другие (2)");
        const exact = Breakdown.modelBreakdown("en", models, null);
        compare(exact.rows.length, 5);
        compare(exact.partial, false);
        compare(Breakdown.modelBreakdown("ru", [
            {
                model: "claude-next",
                totalTokens: 412000,
                costMicros: 0,
                partial: true
            }
        ], null).rows[0].cost, "без цены");
        compare(Breakdown.modelBreakdown("en", [], other), null);
        compare(Breakdown.totalLine("en", {
            costMicros: 1234567890,
            totalTokens: 1203448
        }), "$1,234.57 · 1,203,448 tokens");
    }

    function test_trend() {
        const days = Trend.lastDays([
            {
                date: "2026-09-21",
                totalTokens: 1200000,
                costMicros: 3400000
            }
        ]);
        compare(days.length, 30);
        compare(days[29].totalTokens, 1200000);
        compare(Trend.barShare(1, 100), 0.18);
        compare(Trend.barShare(0, 100), 0);
        compare(Trend.peakDescription("en", days), "Peak 1.2M tokens on Sep 21");
        compare(Trend.peakDescription("ru", days), "Пик: 1.2M токенов, 21 сен");
        compare(Format.dayTooltip("en", days[29]), "Sep 21 · 1.2M tokens · $3.40");
        compare(Format.dayTooltip("ru", days[29]), "21 сен · 1.2M токенов · $3.40");
        compare(Format.dayTooltip("en", days[0]), "");
        compare(Trend.indexAt(9, 5, 30), 1);
        compare(Trend.indexAt(160, 5, 30), -1);
        compare(Trend.indexAt(-1, 5, 30), -1);
    }
}
