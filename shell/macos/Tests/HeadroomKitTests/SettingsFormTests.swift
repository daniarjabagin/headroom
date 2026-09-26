import Foundation
import XCTest

@testable import HeadroomKit

final class SettingsFormTests: XCTestCase {
    func testNewSettingsTextsHaveBothLanguages() {
        check(LayoutSettingsText.self)
        check(MenuBarSettingsText.self)
        check(SpendSettingsText.self)
        check(CardsSettingsText.self)
        check(RefreshSettingsText.self)
        check(PrivacySettingsText.self)
        check(KeyboardSettingsText.self)
        check(AlertSettingsText.self)
        check(AdvancedText.self)
        check(OnboardingText.self)
        check(SignInAgainText.self)
        check(SupportText.self)
    }

    func testSupportLinkPointsAtTheRepository() {
        XCTAssertEqual(SupportLink.repository?.absoluteString, "https://github.com/daniarjabagin/headroom")
        XCTAssertNil(SupportLink.repository?.query)
        XCTAssertEqual(UIStrings(language: .ru).text(SupportText.openGitHub), "Открыть GitHub")
    }

    func testAlmostOutDetailNamesTheThreshold() {
        XCTAssertEqual(
            UIStrings(language: .en).fill(AlertSettingsText.almostOutDetail, ["percent": "20"]),
            "A limit drops under 20% left")
        XCTAssertEqual(
            UIStrings(language: .ru).fill(AlertSettingsText.almostOutDetail, ["percent": "5"]),
            "Остаток лимита опускается ниже 5%")
    }

    func testTimeOfDayRoundTripsThroughAPickerDate() throws {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = try XCTUnwrap(TimeZone(identifier: "Asia/Almaty"))
        let day = Date(timeIntervalSince1970: 1_790_000_000)
        let time = try XCTUnwrap(TimeOfDay("23:30"))
        let date = time.date(on: day, calendar: calendar)
        XCTAssertEqual(calendar.component(.hour, from: date), 23)
        XCTAssertEqual(calendar.component(.minute, from: date), 30)
        XCTAssertEqual(TimeOfDay(date: date, calendar: calendar), time)
        XCTAssertEqual(TimeOfDay(date: date.addingTimeInterval(45), calendar: calendar), time)
    }

    func testAlertProvidersAreDistinctInAccountOrder() throws {
        let state = try Build.state(accounts: [
            Build.accountJSON(id: "codex:a"), Build.accountJSON(id: "claude:a", provider: "claude"),
            Build.accountJSON(id: "codex:b"),
        ])
        XCTAssertEqual(
            SettingsOptions.alertProviders(state.accounts),
            [AlertProvider(id: "codex", name: "Codex"), AlertProvider(id: "claude", name: "Claude")])
    }

    func testLoginCommandSignsTheSameAccountInAgain() {
        XCTAssertEqual(
            AccountCommand.login(accountID: "claude:1a", apiKeyOnStdin: false).arguments,
            ["accounts", "login", "claude:1a", "--progress", "json"])
        XCTAssertEqual(
            AccountCommand.login(accountID: "openrouter:2b", apiKeyOnStdin: true).arguments,
            ["accounts", "login", "openrouter:2b", "--api-key-stdin", "--progress", "json"])
    }

    @MainActor
    func testSessionForAnExistingAccountRunsLoginAndIgnoresTheLabel() {
        let provider = ProviderInfo(
            id: "claude", displayName: "Claude", addAccount: [.cliLogin(program: "claude")], multiAccount: true,
            localUsage: true, links: nil)
        let launcher = FakeHelperLauncher()
        let session = AddAccountSession(provider: provider, launcher: launcher, accountID: "claude:1a")
        session.start(label: "Work")
        XCTAssertEqual(launcher.commands, [.login(accountID: "claude:1a", apiKeyOnStdin: false)])
    }

    @MainActor
    func testSignInAgainRoutesToSettings() {
        let model = AppModel(preferredLanguages: ["en"])
        var routes: [SettingsRoute] = []
        model.settingsPresenter = { routes.append($0) }
        model.signInAgain(accountID: "claude:1a", provider: "claude")
        XCTAssertEqual(routes, [.signInAgain(accountID: "claude:1a", provider: "claude")])
    }

    private func check<Key: LocalizedText>(_ type: Key.Type) {
        for key in Key.allCases {
            let (english, russian) = key.translations
            XCTAssertFalse(english.isEmpty, "\(Key.self).\(key)")
            XCTAssertFalse(russian.isEmpty, "\(Key.self).\(key)")
            XCTAssertNotEqual(english, russian, "\(Key.self).\(key)")
            XCTAssertEqual(placeholders(english), placeholders(russian), "\(Key.self).\(key)")
        }
    }

    private func placeholders(_ text: String) -> [Substring] {
        text.split(separator: "{").dropFirst().compactMap { $0.split(separator: "}").first }.sorted()
    }
}
