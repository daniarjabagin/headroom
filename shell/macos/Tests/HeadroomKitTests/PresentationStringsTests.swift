import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationStringsTests: XCTestCase {
    func testEveryPopupStringIsTranslated() {
        for key in PopupText.allCases {
            XCTAssertFalse(key.english.isEmpty)
            XCTAssertFalse(key.russian.isEmpty)
        }
        for template in PopupTemplate.allCases {
            XCTAssertEqual(placeholders(template.english), placeholders(template.russian), "\(template)")
        }
    }

    func testFillReplacesPlaceholders() {
        XCTAssertEqual(Build.english.strings.fill(.signedOutOf, ["provider": "Claude"]), "Signed out of Claude")
        XCTAssertEqual(Build.russian.strings.fill(.couldNotRefresh, ["provider": "Codex"]), "Не удалось обновить Codex")
        XCTAssertEqual(SpendUnit.cost.title(Build.russian.strings), "Всего потрачено")
    }

    func testNoticeTranslation() {
        let translate = { (text: String) in NoticeTranslation.translate(text, language: .ru) }
        XCTAssertEqual(translate("Weekly limit shared with Codex Cloud"), "Недельный лимит общий с Codex Cloud")
        XCTAssertEqual(translate("Max 5x renews on 2026-10-01 (UTC)."), "Max 5x продлевается 2026-10-01 (UTC).")
        XCTAssertEqual(translate("Pro ends on 2026-10-01 (UTC)."), "Pro заканчивается 2026-10-01 (UTC).")
        XCTAssertEqual(translate("Kilo credits are used up"), "Кредиты Kilo закончились")
        XCTAssertEqual(translate("Unlimited credits"), "Безлимитные кредиты")
        XCTAssertEqual(translate("No monthly credits on this plan"), "В этом тарифе нет ежемесячных кредитов")
        XCTAssertEqual(translate("Balance is not enough for API calls"), "Баланса не хватает для вызовов API")
        XCTAssertEqual(
            translate("Balance is used up; API calls fail until you top up"),
            "Баланс исчерпан — вызовы API не пройдут, пока вы не пополните счёт")
        XCTAssertEqual(
            translate("Balance is used up; API requests fail until you top up"),
            "Баланс исчерпан — запросы к API не пройдут, пока вы не пополните счёт")
        XCTAssertEqual(
            translate("Cash balance is negative: the account is in debt"),
            "Денежный баланс отрицательный — на счёте долг")
        XCTAssertEqual(translate("Something new"), "Something new")
        XCTAssertEqual(
            NoticeTranslation.translate("No Cline credits left.", language: .en), "No Cline credits left.")
    }

    func testLabelTranslation() {
        let translate = { (label: String) in LabelTranslation.translate(label, language: .ru) }
        let labels = [
            "Balance", "Vouchers", "Cash", "Credit balance", "Organization credits", "Point balance", "Bonus credits",
            "Monthly credits", "Credits", "Extra usage",
        ]
        XCTAssertEqual(
            labels.map(translate),
            [
                "Баланс", "Ваучеры", "Денежный баланс", "Кредиты", "Кредиты организации", "Баланс баллов",
                "Бонусные кредиты", "Кредиты на месяц", "Кредиты", "Доп. использование",
            ])
        XCTAssertEqual(translate("Spent today"), "Spent today")
        XCTAssertEqual(LabelTranslation.translate("Organization credits", language: .en), "Organization credits")
        XCTAssertEqual(Build.russian.windowLabel(id: "monthly", label: "Monthly credits"), "Кредиты на месяц")
    }

    private func placeholders(_ text: String) -> Set<String> {
        var result = Set<String>()
        var rest = text[...]
        while let open = rest.firstIndex(of: "{"), let close = rest[open...].firstIndex(of: "}") {
            result.insert(String(rest[open...close]))
            rest = rest[rest.index(after: close)...]
        }
        return result
    }
}
