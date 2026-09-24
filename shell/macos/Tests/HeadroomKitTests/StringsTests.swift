import Foundation
import XCTest

@testable import HeadroomKit

final class StringsTests: XCTestCase {
    private static let sameInBothLanguages: Set<String> = ["Headroom"]

    func testEveryKeyHasBothLanguages() {
        check(SettingsText.self)
        check(MenuText.self)
        check(AppearanceText.self)
        check(MenuBarText.self)
        check(NotificationText.self)
        check(ServiceText.self)
        check(AccountsText.self)
        check(RemovalText.self)
        check(AddAccountText.self)
        check(SignInText.self)
        check(UpdateText.self)
        check(WelcomeText.self)
        for key in UIText.allCases {
            assertTranslated(String(describing: key), english: key.english, russian: key.russian)
        }
    }

    func testPluralTemplatesHaveEveryForm() {
        for template in PluralTemplate.allCases {
            XCTAssertEqual(template.forms(.en).count, 2)
            XCTAssertEqual(template.forms(.ru).count, 3)
            let placeholders = (template.forms(.en) + template.forms(.ru)).map(Self.placeholders)
            XCTAssertTrue(placeholders.allSatisfy { $0 == ["count"] }, "\(template)")
        }
    }

    func testRefreshIntervalLabels() {
        let english = UIStrings(language: .en)
        let russian = UIStrings(language: .ru)
        XCTAssertEqual(english.refreshInterval(seconds: 60), "Every minute")
        XCTAssertEqual(english.refreshInterval(seconds: 300), "Every 5 minutes")
        XCTAssertEqual(english.refreshInterval(seconds: 90), "Every 90 seconds")
        XCTAssertEqual(russian.refreshInterval(seconds: 60), "Каждую минуту")
        XCTAssertEqual(russian.refreshInterval(seconds: 120), "Каждые 2 минуты")
        XCTAssertEqual(russian.refreshInterval(seconds: 1800), "Каждые 30 минут")
        XCTAssertEqual(russian.refreshInterval(seconds: 1260), "Каждую 21 минуту")
        XCTAssertEqual(russian.refreshInterval(seconds: 61), "Каждую 61 секунду")
    }

    func testFillSubstitutesPlaceholders() {
        let strings = UIStrings(language: .ru)
        XCTAssertEqual(strings.fill(RemovalText.removeTitle, ["name": "Codex"]), "Удалить «Codex»?")
        XCTAssertEqual(
            UIStrings(language: .en).fill(RemovalText.removeCLIBody, ["provider": "Claude"]),
            "Headroom will stop showing this account. The Claude CLI stays signed in; you can sign in again through Headroom."
        )
    }

    private func check<Key: LocalizedText>(_ type: Key.Type) {
        for key in Key.allCases {
            let (english, russian) = key.translations
            assertTranslated("\(Key.self).\(key)", english: english, russian: russian)
        }
    }

    private func assertTranslated(_ name: String, english: String, russian: String) {
        XCTAssertFalse(english.isEmpty, "\(name) has no English text")
        XCTAssertFalse(russian.isEmpty, "\(name) has no Russian text")
        XCTAssertEqual(Self.placeholders(english), Self.placeholders(russian), "\(name) placeholders differ")
        if !Self.sameInBothLanguages.contains(english) {
            XCTAssertNotEqual(english, russian, "\(name) is not translated")
        }
    }

    private static func placeholders(_ text: String) -> Set<String> {
        var found: Set<String> = []
        var rest = Substring(text)
        while let open = rest.firstIndex(of: "{"), let close = rest[open...].firstIndex(of: "}") {
            found.insert(String(rest[rest.index(after: open)..<close]))
            rest = rest[rest.index(after: close)...]
        }
        return found
    }
}
