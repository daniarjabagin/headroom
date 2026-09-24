import Foundation
import XCTest

@testable import HeadroomKit

final class ModelDecodingTests: XCTestCase {
    func testFullStateDecodesEveryField() throws {
        let state = try Fixture.decode(DaemonState.self, "state_full")
        XCTAssertEqual(state.version, 1)
        XCTAssertEqual(state.generatedAt, try Fixture.timestamp("2026-09-23T10:00:00Z"))
        XCTAssertFalse(state.offline)
        XCTAssertEqual(state.display.valueMode, .left)
        XCTAssertEqual(state.display.hiddenWindows, ["codex:work": ["weekly"]])
        XCTAssertEqual(state.headline?.accountID, "claude:main")
        XCTAssertEqual(state.headline?.tone, .critical)
        XCTAssertEqual(state.headline?.remainingPercent, 8.0)
        XCTAssertEqual(state.accounts.map(\.id), ["codex:work", "claude:main", "codex:hidden"])
    }

    func testMoneyBalancesDecodeCurrencyAndMicros() throws {
        let balances = try Fixture.decode([Balance].self, "balances")
        XCTAssertEqual(balances.map(\.kind), [.usd, .money, .money, .money, .money, .money, .count, .unknown])
        XCTAssertEqual(balances[1].currency, "CNY")
        XCTAssertEqual(balances[1].micros, 12_500_000)
        XCTAssertNil(balances[1].usdMicros)
        XCTAssertEqual(balances[2].micros, -3_000_000)
        XCTAssertEqual(balances[5].currency, "EUR")
        XCTAssertNil(balances[5].micros)
        XCTAssertNil(balances[0].currency)
    }

    func testAccountsWindowsPaceAndNotices() throws {
        let state = try Fixture.decode(DaemonState.self, "state_full")
        let codex = state.accounts[0]
        XCTAssertEqual(codex.status, .fresh)
        XCTAssertEqual(codex.owner, .cli)
        XCTAssertEqual(codex.source, .live)
        XCTAssertEqual(codex.balances.first?.usdMicros, 12_500_000)
        XCTAssertEqual(codex.balances.first?.kind, .usd)
        XCTAssertEqual(codex.notices, [Notice(tone: .warning, text: "Weekly limit shared with Codex Cloud")])
        let session = try XCTUnwrap(codex.windows.first)
        XCTAssertEqual(session.periodSeconds, 18_000)
        XCTAssertEqual(session.pace.severity, .close)
        XCTAssertEqual(session.pace.evenPacePercent, 60.0)
        XCTAssertNil(session.pace.runsOutAt)
        XCTAssertTrue(codex.windows[1].hidden)
    }

    func testSignedOutAccountKeepsErrorAndNanosecondRunOut() throws {
        let state = try Fixture.decode(DaemonState.self, "state_full")
        let claude = state.accounts[1]
        XCTAssertEqual(claude.status, .signedOut)
        XCTAssertEqual(claude.error?.kind, "sign_in_expired")
        XCTAssertEqual(claude.displayName, "ada@claude.example")
        let runsOut = try XCTUnwrap(claude.windows.first?.pace.runsOutAt)
        let expected = try Fixture.timestamp("2026-09-23T10:23:28Z").date.addingTimeInterval(0.695652174)
        XCTAssertEqual(runsOut.date.timeIntervalSince1970, expected.timeIntervalSince1970, accuracy: 1e-6)
    }

    func testUsageAndSpendKeepIntegerTotals() throws {
        let state = try Fixture.decode(DaemonState.self, "state_full")
        let codexUsage = try XCTUnwrap(state.usage.first)
        XCTAssertEqual(codexUsage.today.tokens.total, 1200)
        XCTAssertEqual(codexUsage.today.costUSDMicros, 2400)
        XCTAssertEqual(codexUsage.today.models.first?.model, "gpt-5.5")
        XCTAssertEqual(codexUsage.daily.count, 30)
        XCTAssertEqual(state.spend.today.costUSDMicros, 12_400)
        XCTAssertEqual(state.spend.today.totalTokens, 6_200)
        XCTAssertEqual(state.spend.today.byProvider.map(\.provider), ["claude", "codex"])
    }

    func testEmptyState() throws {
        let state = try Fixture.decode(DaemonState.self, "state_empty")
        XCTAssertNil(state.headline)
        XCTAssertNil(state.nextRefreshAt)
        XCTAssertTrue(state.accounts.isEmpty)
        XCTAssertEqual(state.spend.last30Days.byProvider, [])
    }

    func testNoSubscriptionAccount() throws {
        let account = try Fixture.decode(Account.self, "account_no_subscription")
        XCTAssertEqual(account.status, .noSubscription)
        XCTAssertNil(account.plan)
        XCTAssertNil(account.updatedAt)
        XCTAssertNil(account.source)
        XCTAssertEqual(account.displayName, "Work")
    }

    func testProvidersRegistry() throws {
        let payload = try Fixture.decode(ProvidersPayload.self, "providers")
        XCTAssertEqual(payload.version, 1)
        XCTAssertEqual(payload.providers.first?.addAccount, [.cliLogin(program: "codex")])
        let opencode = try XCTUnwrap(payload.providers.first { $0.id == "opencode" })
        XCTAssertEqual(opencode.addAccount.count, 2)
        guard case .apiKey(let label, let consoleURL, _) = opencode.addAccount[0] else {
            return XCTFail("expected api_key first")
        }
        XCTAssertEqual(label, "API key")
        XCTAssertEqual(consoleURL, "https://opencode.ai/auth")
    }

    func testUnknownAddAccountKindIsKept() throws {
        let method = try Fixture.decode(AddAccountMethod.self, json: #"{"kind":"oauth_device","x":1}"#)
        XCTAssertEqual(method, .unsupported(kind: "oauth_device"))
    }

    func testUnknownFieldsAndEnumValuesDoNotBreakDecoding() throws {
        let json = #"{"tone":"ultraviolet","text":"hi","future":true}"#
        XCTAssertEqual(try Fixture.decode(Notice.self, json: json), Notice(tone: .neutral, text: "hi"))
    }

    func testMissingRequiredFieldFails() {
        XCTAssertThrowsError(try Fixture.decode(Notice.self, json: #"{"tone":"good"}"#))
    }

    func testInvalidTimestampFails() {
        let json = #"{"severity":"healthy","runs_out_at":"yesterday"}"#
        XCTAssertThrowsError(try Fixture.decode(Pace.self, json: json))
    }

    func testAlertNotificationParams() throws {
        let json = #"""
            {"id":"codex:1/session/almost_out","title":"Codex · Work — Session","body":"Under 10% left","account_id":"codex:1","urgency":"critical"}
            """#
        let alert = try Fixture.decode(DaemonAlert.self, json: json)
        XCTAssertEqual(alert.urgency, .critical)
        XCTAssertEqual(alert.accountID, "codex:1")
    }
}
