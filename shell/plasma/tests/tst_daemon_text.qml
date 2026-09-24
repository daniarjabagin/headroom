import QtQuick
import QtTest
import "../package/contents/ui/logic/Account.js" as Account
import "../package/contents/ui/logic/DaemonText.js" as DaemonText
import "../package/contents/ui/logic/Format.js" as Format

TestCase {
    function test_balance_labels_in_russian() {
        const labels = ["Balance", "Vouchers", "Cash", "Credit balance", "Organization credits", "Point balance", "Bonus credits", "Credits", "Extra usage"];
        compare(labels.map(text => DaemonText.label("ru", text)), ["Баланс", "Ваучеры", "Денежный баланс", "Кредиты", "Кредиты организации", "Баланс баллов", "Бонусные кредиты", "Кредиты", "Доп. использование"]);
    }

    function test_labels_stay_english_and_unknown_labels_pass_through() {
        compare(DaemonText.label("en", "Organization credits"), "Organization credits");
        compare(DaemonText.label("ru", "Spent today"), "Spent today");
        compare(DaemonText.label("ru", "constructor"), "constructor");
        compare(DaemonText.label("ru", null), null);
    }

    function test_window_labels_translate_by_text() {
        compare(Format.windowLabel("ru", {
            id: "monthly",
            label: "Monthly credits"
        }), "Кредиты на месяц");
        compare(Format.shortWindowLabel("ru", "monthly", "Monthly credits"), "Кредиты на месяц");
        compare(Format.shortWindowLabel("ru", "monthly", null), "monthly");
    }

    function test_new_provider_notices_in_russian_data() {
        return [
            {
                tag: "kilo",
                text: "Kilo credits are used up",
                russian: "Кредиты Kilo закончились"
            },
            {
                tag: "warp unlimited",
                text: "Unlimited credits",
                russian: "Безлимитные кредиты"
            },
            {
                tag: "warp none",
                text: "No monthly credits on this plan",
                russian: "В этом тарифе нет ежемесячных кредитов"
            },
            {
                tag: "deepseek short",
                text: "Balance is not enough for API calls",
                russian: "Баланса не хватает для вызовов API"
            },
            {
                tag: "deepseek used up",
                text: "Balance is used up; API calls fail until you top up",
                russian: "Баланс исчерпан — вызовы API не пройдут, пока вы не пополните счёт"
            },
            {
                tag: "moonshot used up",
                text: "Balance is used up; API requests fail until you top up",
                russian: "Баланс исчерпан — запросы к API не пройдут, пока вы не пополните счёт"
            },
            {
                tag: "moonshot debt",
                text: "Cash balance is negative: the account is in debt",
                russian: "Денежный баланс отрицательный — на счёте долг"
            },
            {
                tag: "codex cloud",
                text: "Weekly limit shared with Codex Cloud",
                russian: "Недельный лимит общий с Codex Cloud"
            }
        ];
    }

    function test_new_provider_notices_in_russian(data) {
        compare(DaemonText.notice("ru", data.text), data.russian);
        compare(DaemonText.notice("en", data.text), data.text);
    }

    function test_patterned_notices() {
        compare(DaemonText.notice("ru", "Extra usage on, cap $25.00"), "Доп. использование включено, предел $25.00");
        compare(DaemonText.notice("ru", "Cline Pro renews on 2026-10-01 (UTC)."), "Cline Pro продлевается 2026-10-01 (UTC).");
        compare(DaemonText.notice("ru", "Pro ends on 2026-10-01 (UTC)."), "Pro заканчивается 2026-10-01 (UTC).");
        compare(DaemonText.notice("ru", "Something new"), "Something new");
        compare(DaemonText.notice("en", "Extra usage on, cap $25.00"), "Extra usage on, cap $25.00");
    }

    function test_account_notices_are_translated() {
        const account = {
            id: "kilo:1",
            provider: "kilo",
            providerName: "Kilo Code",
            status: "ok",
            error: null,
            notices: [
                {
                    tone: "critical",
                    text: "Kilo credits are used up"
                }
            ]
        };
        compare(Account.notices("ru", account, false, []).map(row => row.title), ["Кредиты Kilo закончились"]);
    }

    name: "DaemonText"
}
