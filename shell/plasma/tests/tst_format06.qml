import QtQuick
import QtTest
import "../package/contents/ui/logic/FormatSpend.js" as FormatSpend
import "../package/contents/ui/logic/FormatTime.js" as FormatTime
import "../package/contents/ui/logic/State.js" as State
import "../package/contents/ui/logic/Summary.js" as Summary

TestCase {
    name: "Format06"

    function localDate(day, hours, minutes) {
        return new Date(2026, 8, day, hours, minutes);
    }

    function test_hour_cycle_data() {
        return [
            {
                tag: "forced 12h",
                format: "12h",
                pattern: "HH:mm",
                cycle: "12h"
            },
            {
                tag: "forced 24h",
                format: "24h",
                pattern: "h:mm AP",
                cycle: "24h"
            },
            {
                tag: "en_US",
                format: "auto",
                pattern: "h:mm AP",
                cycle: "12h"
            },
            {
                tag: "lowercase meridiem",
                format: "auto",
                pattern: "h:mm ap",
                cycle: "12h"
            },
            {
                tag: "ru_RU",
                format: "auto",
                pattern: "HH:mm",
                cycle: "24h"
            },
            {
                tag: "quoted letters",
                format: "auto",
                pattern: "HH'h'mm 'Uhr am'",
                cycle: "24h"
            },
            {
                tag: "missing pattern",
                format: undefined,
                pattern: undefined,
                cycle: "24h"
            }
        ];
    }

    function test_hour_cycle(row) {
        compare(FormatTime.hourCycle(row.format, row.pattern), row.cycle);
    }

    function test_locale_hour_cycle_follows_qt_locale() {
        const expected = FormatTime.hourCycle("auto", Qt.locale().timeFormat(Locale.ShortFormat));
        compare(FormatTime.localeHourCycle("auto"), expected);
        compare(FormatTime.localeHourCycle(undefined), expected);
        compare(FormatTime.localeHourCycle("12h"), "12h");
        compare(FormatTime.localeHourCycle("24h"), "24h");
    }

    function test_clock_time_data() {
        return [
            {
                tag: "morning 24h",
                date: localDate(23, 9, 5),
                format: "24h",
                text: "09:05"
            },
            {
                tag: "morning 12h",
                date: localDate(23, 9, 5),
                format: "12h",
                text: "9:05 AM"
            },
            {
                tag: "midnight 12h",
                date: localDate(23, 0, 30),
                format: "12h",
                text: "12:30 AM"
            },
            {
                tag: "noon 12h",
                date: localDate(23, 12, 0),
                format: "12h",
                text: "12:00 PM"
            },
            {
                tag: "evening 12h",
                date: localDate(23, 17, 45),
                format: "12h",
                text: "5:45 PM"
            },
            {
                tag: "evening 24h",
                date: localDate(23, 17, 45),
                format: "24h",
                text: "17:45"
            }
        ];
    }

    function test_clock_time(row) {
        compare(FormatTime.clockTime(row.date, row.format), row.text);
    }

    function test_exact_resets_follow_time_format() {
        const base = localDate(23, 10, 0);
        compare(FormatTime.resetText("en", localDate(23, 17, 5), base, "exact", false, "12h"), "Resets today at 5:05 PM");
        compare(FormatTime.resetText("ru", localDate(24, 9, 30), base, "exact", false, "12h"), "Сброс завтра в 9:30 AM");
        compare(FormatTime.resetText("en", localDate(26, 20, 0), base, "exact", false, "24h"), "Resets Sat at 20:00");
        compare(FormatTime.resetText("en", localDate(23, 12, 41), base, "countdown", false, "12h"), "Resets in 2h 41m");
    }

    function test_offline_footer_uses_display_time_format() {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("../dev/sample-state.json"), false);
        request.send();
        const raw = JSON.parse(request.responseText);
        raw.offline = true;
        raw.last_success_at = localDate(23, 14, 7).toISOString();
        raw.display.time_format = "12h";
        const view = {
            kind: "ready",
            state: State.parseState(JSON.stringify(raw))
        };
        compare(Summary.footerLine("en", view, localDate(23, 15, 0)).text, "Offline — last update 2:07 PM");
        raw.display.time_format = "24h";
        view.state = State.parseState(JSON.stringify(raw));
        compare(Summary.footerLine("en", view, localDate(23, 15, 0)).text, "Offline — last update 14:07");
    }

    function test_cost_per_mtok() {
        compare(FormatSpend.costPerMtok("en", 3756938), "$3.76/MTok");
        compare(FormatSpend.costPerMtok("ru", 217163), "$0.22/млн ток.");
        compare(FormatSpend.costPerMtok("en", 1500000000), "$1.50K/MTok");
        compare(FormatSpend.costPerMtok("en", null), "—");
    }

    function test_share_text_follows_language_decimal_separator() {
        compare(FormatSpend.shareText("en", 618), "61.8%");
        compare(FormatSpend.shareText("ru", 618), "61,8%");
        compare(FormatSpend.shareText("ru", 1000), "100,0%");
        compare(FormatSpend.shareText("en", 0), "0.0%");
    }

    function test_unit_values() {
        const totals = {
            costMicros: 18420000,
            totalTokens: 48100000,
            costPerMtokMicros: 382952
        };
        compare(FormatSpend.unitValue(totals, "cost"), "$18.42");
        compare(FormatSpend.unitValue(totals, "tokens"), "48.1M");
        compare(FormatSpend.unitValue(totals, "cost_per_mtok"), "$0.38");
        compare(FormatSpend.unitValue(Object.assign({}, totals, {
            costPerMtokMicros: null
        }), "cost_per_mtok"), "—");
        compare(FormatSpend.tokensText("en", 48100000), "48.1M tokens");
        compare(FormatSpend.tokensText("ru", 950), "950 токенов");
    }

    function test_middle_ellipsis_data() {
        return [
            {
                tag: "fits",
                path: "~/code/headroom",
                max: 20,
                text: "~/code/headroom"
            },
            {
                tag: "keeps last segment",
                path: "~/code/clients/acme/very-long-folder/headroom",
                max: 24,
                text: "~/code/clients…/headroom"
            },
            {
                tag: "long last segment",
                path: "~/code/an-extremely-long-project-name",
                max: 16,
                text: "~/code/a…ct-name"
            },
            {
                tag: "no slash",
                path: "abcdefghijklmnopqrstuvwxyz",
                max: 10,
                text: "abcde…wxyz"
            },
            {
                tag: "unicode",
                path: "~/проекты/очень-длинная-папка/хедрум",
                max: 20,
                text: "~/проекты/оч…/хедрум"
            }
        ];
    }

    function test_middle_ellipsis(row) {
        const text = FormatSpend.middleEllipsis(row.path, row.max);
        compare(text, row.text);
        verify(Array.from(text).length <= row.max);
    }

    function test_project_labels() {
        compare(FormatSpend.projectLabel("en", null, 20), "No project");
        compare(FormatSpend.projectLabel("ru", null, 20), "Без проекта");
        compare(FormatSpend.projectLabel("en", "~/code/headroom", 20), "~/code/headroom");
    }
}
