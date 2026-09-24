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
        XCTAssertEqual(Build.russian.strings.text(.totalSpend), "Всего потрачено")
    }

    func testNoticeTranslation() {
        let translate = { (text: String) in NoticeTranslation.translate(text, language: .ru) }
        XCTAssertEqual(translate("Weekly limit shared with Codex Cloud"), "Недельный лимит общий с Codex Cloud")
        XCTAssertEqual(translate("Max 5x renews on 2026-10-01 (UTC)."), "Max 5x продлевается 2026-10-01 (UTC).")
        XCTAssertEqual(translate("Pro ends on 2026-10-01 (UTC)."), "Pro заканчивается 2026-10-01 (UTC).")
        XCTAssertEqual(translate("Something new"), "Something new")
        XCTAssertEqual(
            NoticeTranslation.translate("No Cline credits left.", language: .en), "No Cline credits left.")
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
