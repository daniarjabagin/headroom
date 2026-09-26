import Foundation
import XCTest

@testable import HeadroomKit

final class PopupRelease061Tests: XCTestCase {
    private static let cliLogin =
        #"{"action":"cli_login","command":"claude auth login --claudeai","account_id":"claude:main"}"#

    func testShowBreakdownDefaultsToTrueAndDecodes() throws {
        XCTAssertTrue(try Build.display().showBreakdown)
        let hidden = try Build.mutated("state_full") { object in
            var display = object["display"] as? [String: Any] ?? [:]
            display["show_breakdown"] = false
            object["display"] = display
        }
        XCTAssertFalse(hidden.display.showBreakdown)
    }

    func testHiddenBreakdownLeavesTheCardWithoutTheList() throws {
        let spend = try SpendSamples.spend(SpendSamples.twoProviders())
        let selection = SpendSelection(period: .last30Days, unit: .cost, breakdown: .models)
        let shown = SpendCardModel.make(spend: spend, selection: selection, formatter: Build.english)
        XCTAssertNotNil(shown.breakdown)
        let hidden = SpendCardModel.make(
            spend: spend, selection: selection, formatter: Build.english, showBreakdown: false)
        XCTAssertNil(hidden.breakdown)
        XCTAssertEqual(hidden.entries, shown.entries)
    }

    func testShowBreakdownChangeIsASpendSetting() throws {
        let change = SettingsChange.spend(.showBreakdown(false))
        XCTAssertEqual(try RPCCodec.encodeString(change.patch), #"{"display":{"show_breakdown":false}}"#)
        let base = try Fixture.decode(Settings.self, json: SettingsTests.release06)
        XCTAssertFalse(change.applied(to: base).display.showBreakdown)
        XCTAssertTrue(SettingsChange.spend(.showBreakdown(true)).applied(to: base).display.showBreakdown)
        XCTAssertTrue(change.requiresRelease06)
    }

    func testOtherModelsCarryTheDaemonRate() throws {
        let other = try Fixture.decode(
            OtherModels.self,
            json: #"{"count":2,"total_tokens":1900000,"cost_usd_micros":1490000,"partial":false,"#
                + #""cost_per_mtok_usd_micros":784211}"#)
        XCTAssertEqual(other.costPerMTokUSDMicros, 784_211)
        let old = try Fixture.decode(
            OtherModels.self, json: #"{"count":2,"total_tokens":1900000,"cost_usd_micros":1490000,"partial":false}"#)
        XCTAssertNil(old.costPerMTokUSDMicros)
    }

    func testCLILoginWithAccountDecodes() throws {
        let recovery = try Fixture.decode(AccountRecovery.self, json: Self.cliLogin)
        XCTAssertEqual(recovery, .cliLogin(command: "claude auth login --claudeai", accountID: "claude:main"))
        XCTAssertEqual(recovery.signInAccountID, "claude:main")
        XCTAssertEqual(AccountRecovery.signIn(accountID: "a").signInAccountID, "a")
        XCTAssertNil(AccountRecovery.cliLogin(command: "gh auth login", accountID: nil).signInAccountID)
        XCTAssertNil(AccountRecovery.retry.signInAccountID)
    }

    func testCLILoginWithAccountSignsInAndKeepsTheCommand() throws {
        let account = Build.accountJSON(
            id: "claude:main", provider: "claude", status: "signed_out",
            error: #"{"kind":"sign_in_expired","message":"sign-in expired"}"#, recovery: Self.cliLogin)
        let section = AccountSectionModel.sections(try Build.state(accounts: [account]), formatter: Build.english)[0]
        guard case .blocked(let notice) = section.body else { return XCTFail("expected a blocking notice") }
        XCTAssertEqual(
            notice.recovery.primary, .cliSignIn(provider: "claude", command: "claude auth login --claudeai"))
        XCTAssertEqual(notice.detail, "Run `claude auth login --claudeai` in Terminal, then press Retry.")
        XCTAssertEqual(UIStrings(language: .en).text(PopupText.signIn), "Sign in")
        XCTAssertEqual(UIStrings(language: .ru).text(PopupText.signIn), "Войти")
    }

    func testBrandName() {
        XCTAssertEqual(BrandMark.name, "Headroom")
    }
}
