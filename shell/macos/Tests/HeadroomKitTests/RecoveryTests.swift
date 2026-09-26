import Foundation
import XCTest

@testable import HeadroomKit

final class RecoveryTests: XCTestCase {
    private static let expired = #"{"kind":"sign_in_expired","message":"sign-in expired"}"#
    private static let network = #"{"kind":"network","message":"HTTP 503"}"#
    private static let cliLogin = #"{"action":"cli_login","command":"claude auth login --claudeai"}"#

    func testRecoveryActionsDecode() throws {
        let cases: [(String, AccountRecovery)] = [
            (#"{"action":"retry"}"#, .retry),
            (#"{"action":"sign_in","account_id":"claude:1a"}"#, .signIn(accountID: "claude:1a")),
            (Self.cliLogin, .cliLogin(command: "claude auth login --claudeai", accountID: nil)),
            (#"{"action":"open_browser","url":"https://example.com"}"#, .retry),
        ]
        for (json, expected) in cases {
            XCTAssertEqual(try Fixture.decode(AccountRecovery.self, json: json), expected, json)
        }
    }

    func testRecoveryWithoutItsFieldFails() {
        XCTAssertThrowsError(try Fixture.decode(AccountRecovery.self, json: #"{"action":"sign_in"}"#))
        XCTAssertThrowsError(try Fixture.decode(AccountRecovery.self, json: #"{"action":"cli_login"}"#))
    }

    func testFixtureAccountsCarryRecovery() throws {
        let state = try Build.full()
        XCTAssertEqual(state.accounts.map(\.recovery), [nil, .retry, nil])
        XCTAssertTrue(state.accounts.allSatisfy(\.reportsRecovery))
    }

    func testMissingRecoveryIsToleratedAndRemembered() throws {
        let state = try Build.state(accounts: [
            Build.accountJSON(id: "old"), Build.accountJSON(id: "new", recovery: "null"),
        ])
        XCTAssertEqual(state.accounts.map(\.recovery), [nil, nil])
        XCTAssertEqual(state.accounts.map(\.reportsRecovery), [false, true])
    }

    func testErrorNoticeStaysWhileRefreshingAndShowsItsRetryAsBusy() throws {
        let account = Build.accountJSON(
            id: "a", status: "refreshing", error: Self.network, windows: [Build.windowJSON()],
            recovery: #"{"action":"retry"}"#)
        let notice = try errorNotice(account)
        XCTAssertEqual(notice.title, "Couldn't refresh Codex")
        XCTAssertEqual(notice.recovery, NoticeRecovery(accountID: "a", primary: nil, retrying: true))
    }

    func testCLILoginOffersTheCommandAndTellsToRetry() throws {
        let account = Build.accountJSON(
            id: "c", provider: "claude", status: "signed_out", error: Self.expired, recovery: Self.cliLogin)
        let notice = try blocked(account, formatter: Build.english)
        XCTAssertEqual(notice.detail, "Run `claude auth login --claudeai` in Terminal, then press Retry.")
        XCTAssertEqual(notice.recovery.primary, .copyCommand("claude auth login --claudeai"))
        let russian = try blocked(account, formatter: Build.russian)
        XCTAssertEqual(
            russian.detail, "Выполните `claude auth login --claudeai` в Терминале, затем нажмите «Повторить».")
    }

    func testSignInOpensTheProvidersSignIn() throws {
        let account = Build.accountJSON(
            id: "c", provider: "claude", status: "signed_out", error: Self.expired,
            recovery: #"{"action":"sign_in","account_id":"c"}"#)
        let notice = try blocked(account, formatter: Build.english)
        XCTAssertEqual(
            notice.recovery, NoticeRecovery(accountID: "c", primary: .signIn(provider: "claude"), retrying: false))
        XCTAssertEqual(
            notice.detail,
            "Sign in again through Headroom (Settings → Accounts → Add account), or remove the account there.")
    }

    func testAPIKeyOnlyErrorShowsTheCommandInTheErrorNotice() throws {
        let error = #"{"kind":"api_key_only","message":"only an API key is configured"}"#
        let account = Build.accountJSON(id: "a", status: "error", error: error, recovery: Self.cliLogin)
        let notice = try errorNotice(account)
        XCTAssertEqual(notice.note, "Run `claude auth login --claudeai` in Terminal, then press Retry.")
        XCTAssertEqual(notice.recovery?.primary, .copyCommand("claude auth login --claudeai"))
    }

    func testAccountChangedSaysAnotherAccountIsSignedIn() throws {
        let error = #"{"kind":"account_changed","message":"the account at ~/.codex has changed"}"#
        let account = Build.accountJSON(id: "a", status: "error", error: error, recovery: #"{"action":"retry"}"#)
        let notice = try errorNotice(account)
        XCTAssertEqual(notice.title, "Another account is signed in to Codex")
        XCTAssertEqual(notice.detail, "Press Retry to switch to it.")
        XCTAssertEqual(notice.recovery, NoticeRecovery(accountID: "a", primary: nil, retrying: false))
        let russian = try errorNotice(account, formatter: Build.russian)
        XCTAssertEqual(russian.title, "В Codex выполнен вход в другой аккаунт")
        XCTAssertEqual(russian.detail, "Нажмите «Повторить», чтобы переключиться на него.")
    }

    func testWaitingErrorsOfferNoButton() throws {
        let error = #"{"kind":"rate_limited","message":"rate limited"}"#
        let account = Build.accountJSON(id: "a", status: "error", error: error, recovery: "null")
        XCTAssertNil(try errorNotice(account).recovery)
    }

    func testOlderDaemonsKeepRetryAndSignIn() throws {
        let failed = try errorNotice(Build.accountJSON(id: "a", status: "error", error: Self.network))
        XCTAssertEqual(failed.recovery, NoticeRecovery(accountID: "a", primary: nil, retrying: false))
        let signedOut = try blocked(
            Build.accountJSON(id: "b", status: "signed_out", error: Self.expired), formatter: Build.english)
        XCTAssertEqual(signedOut.recovery.primary, .signIn(provider: "codex"))
    }

    func testRefreshingIDs() throws {
        let state = try Build.state(accounts: [
            Build.accountJSON(id: "a", status: "refreshing"), Build.accountJSON(id: "b"),
        ])
        XCTAssertEqual(PopupScreen.refreshingIDs(state), ["a"])
    }

    private func errorNotice(_ account: String, formatter: DisplayFormatter = Build.english) throws -> NoticeModel {
        let section = AccountSectionModel.sections(try Build.state(accounts: [account]), formatter: formatter)[0]
        guard case .limits(let limits) = section.body, let notice = limits.notices.first else {
            throw UnexpectedBody.limits
        }
        return notice
    }

    private func blocked(_ account: String, formatter: DisplayFormatter) throws -> BlockingNotice {
        let section = AccountSectionModel.sections(try Build.state(accounts: [account]), formatter: formatter)[0]
        guard case .blocked(let notice) = section.body else { throw UnexpectedBody.blocked }
        return notice
    }
}

private enum UnexpectedBody: Error {
    case limits, blocked
}
