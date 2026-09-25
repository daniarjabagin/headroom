import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationAccountTests: XCTestCase {
    func testFullStateBuildsVisibleSectionsInDaemonOrder() throws {
        let sections = AccountSectionModel.sections(try Build.full(), formatter: Build.english)
        XCTAssertEqual(sections.map(\.id), ["codex:work", "claude:main"])
        let codex = try XCTUnwrap(sections.first)
        XCTAssertEqual(codex.header, AccountHeaderModel(provider: "codex", title: "Codex", plan: "Pro", status: nil))
        guard case .limits(let limits) = codex.body else { return XCTFail("codex should show limits") }
        XCTAssertEqual(limits.windows.map(\.id), ["session"])
        XCTAssertEqual(limits.notices.map(\.kind), [.warning])
        XCTAssertEqual(limits.notices.first?.title, "Weekly limit shared with Codex Cloud")
        XCTAssertEqual(limits.trend?.count, 30)
        XCTAssertEqual(limits.extras.map(\.title), ["Today", "Yesterday", "Last 30 Days", "Credits"])
        XCTAssertEqual(limits.extras.last?.value, "$12.50")
        XCTAssertTrue(limits.extrasCollapsible)
    }

    func testSectionsFollowTheOptimisticOrder() throws {
        let state = try Build.full()
        let ordered = [state.accounts[1], state.accounts[2], state.accounts[0]]
        let sections = AccountSectionModel.sections(state, ordered: ordered, formatter: Build.english)
        XCTAssertEqual(sections.map(\.id), ["claude:main", "codex:work"])
    }

    func testSignedOutAccountShowsOnlyTheSignInNotice() throws {
        let sections = AccountSectionModel.sections(try Build.full(), formatter: Build.english)
        let claude = try XCTUnwrap(sections.last)
        XCTAssertEqual(
            claude.body,
            .blocked(
                BlockingNotice(
                    kind: .signIn, title: "Signed out of Claude",
                    detail: "Sign in again with the provider's app, then press Retry.",
                    note: "sign-in expired, open the CLI to sign in again",
                    recovery: NoticeRecovery(accountID: "claude:main", primary: nil, retrying: false))))
    }

    func testRetryingSignedOutAccountStaysBlocked() throws {
        let error = #"{"kind":"not_signed_in","message":"not_signed_in"}"#
        let state = try Build.state(accounts: [Build.accountJSON(id: "a", status: "refreshing", error: error)])
        guard case .blocked(let notice) = AccountSectionModel.sections(state, formatter: Build.english)[0].body else {
            return XCTFail("expected a blocking notice")
        }
        XCTAssertEqual(
            notice.recovery, NoticeRecovery(accountID: "a", primary: .signIn(provider: "codex"), retrying: true))
    }

    func testNoSubscriptionHidesPlanAndRepeatsOnlyInformativeMessages() throws {
        let generic = #"{"kind":"no_subscription","message":"No active subscription."}"#
        let specific = #"{"kind":"no_subscription","message":"No active ChatGPT subscription (Free plan)."}"#
        let state = try Build.state(accounts: [
            Build.accountJSON(id: "a", status: "no_subscription", error: generic),
            Build.accountJSON(id: "b", status: "no_subscription", error: specific),
        ])
        let sections = AccountSectionModel.sections(state, formatter: Build.english)
        XCTAssertNil(sections[0].header.plan)
        guard case .blocked(let first) = sections[0].body, case .blocked(let second) = sections[1].body else {
            return XCTFail("expected blocking notices")
        }
        XCTAssertEqual(first.title, "No active subscription")
        XCTAssertNil(first.note)
        XCTAssertEqual(second.note, "No active ChatGPT subscription (Free plan).")
        XCTAssertEqual(second.recovery, NoticeRecovery(accountID: "b", primary: nil, retrying: false))
    }

    func testTitlesNameTheAccountOnlyWhenAProviderHasSeveral() throws {
        let state = try Build.state(accounts: [
            Build.accountJSON(id: "a", label: "\"work\""),
            Build.accountJSON(id: "b", email: "\"ada@example.com\""),
            Build.accountJSON(id: "c", provider: "claude", label: "\"solo\""),
            Build.accountJSON(id: "d", label: "\"hidden\"", hidden: true),
        ])
        let titles = AccountSectionModel.sections(state, formatter: Build.english).map(\.header.title)
        XCTAssertEqual(titles, ["Codex: work", "Codex: ada@example.com", "Claude"])
    }

    func testErrorsShowANoticeUnlessTheWholeDaemonIsOffline() throws {
        let network = #"{"kind":"network","message":"HTTP 503 from chatgpt.com"}"#
        let account = Build.accountJSON(id: "a", status: "error", error: network, windows: [Build.windowJSON()])
        let online = AccountSectionModel.sections(try Build.state(accounts: [account]), formatter: Build.english)[0]
        XCTAssertEqual(online.header.status, .failed(message: "HTTP 503 from chatgpt.com"))
        guard case .limits(let limits) = online.body else { return XCTFail("expected limits") }
        XCTAssertEqual(
            limits.notices.first,
            NoticeModel(
                id: "error", kind: .error, title: "Couldn't refresh Codex", detail: "HTTP 503 from chatgpt.com",
                note: nil, recovery: NoticeRecovery(accountID: "a", primary: nil, retrying: false)))
        let offline = AccountSectionModel.sections(
            try Build.state(accounts: [account], offline: true), formatter: Build.english)[0]
        XCTAssertEqual(offline.header.status, .outdated(updatedAt: nil))
        guard case .limits(let quiet) = offline.body else { return XCTFail("expected limits") }
        XCTAssertTrue(quiet.notices.isEmpty)
    }

    func testHeaderStatusSlot() throws {
        let stamp = "\"2026-09-23T07:00:00Z\""
        let state = try Build.state(accounts: [
            Build.accountJSON(id: "a", status: "refreshing", updatedAt: stamp, windows: [Build.windowJSON()]),
            Build.accountJSON(id: "b", status: "stale", updatedAt: stamp),
            Build.accountJSON(id: "c", status: "fresh"),
        ])
        let statuses = AccountSectionModel.sections(state, formatter: Build.english).map(\.header.status)
        XCTAssertEqual(
            statuses, [.refreshing, .outdated(updatedAt: try Fixture.timestamp("2026-09-23T07:00:00Z")), nil])
    }

    func testFirstRefreshShowsSkeletonRows() throws {
        let state = try Build.state(accounts: [Build.accountJSON(id: "a", status: "refreshing")])
        guard case .limits(let limits) = AccountSectionModel.sections(state, formatter: Build.english)[0].body else {
            return XCTFail("expected limits")
        }
        XCTAssertEqual(limits.skeletonRows, 2)
        XCTAssertFalse(limits.isEmpty)
    }

    func testDaemonNoticesMapToneAndTranslate() throws {
        let notices =
            #"[{"tone":"neutral","text":"Extra usage on, cap $50.00"},{"tone":"critical","text":"No Cline credits left."}]"#
        let state = try Build.state(accounts: [Build.accountJSON(id: "a", notices: notices)])
        guard case .limits(let limits) = AccountSectionModel.sections(state, formatter: Build.russian)[0].body else {
            return XCTFail("expected limits")
        }
        XCTAssertEqual(limits.notices.map(\.kind), [.info, .error])
        XCTAssertEqual(
            limits.notices.map(\.title), ["Доп. использование включено, предел $50.00", "Кредиты Cline закончились."])
    }

    func testExtrasAreInlineWithoutMetersAndHiddenBySettings() throws {
        let balances = #"[{"id":"requests","label":"Requests","kind":"count","value":1500,"unit":"requests"}]"#
        let state = try Build.state(accounts: [Build.accountJSON(id: "a", balances: balances)])
        guard case .limits(let limits) = AccountSectionModel.sections(state, formatter: Build.english)[0].body else {
            return XCTFail("expected limits")
        }
        XCTAssertEqual(limits.extras.map(\.value), ["1,500 requests"])
        XCTAssertFalse(limits.extrasCollapsible)
        let empty = try Build.state(accounts: [Build.accountJSON(id: "a")])
        guard case .limits(let nothing) = AccountSectionModel.sections(empty, formatter: Build.english)[0].body else {
            return XCTFail("expected limits")
        }
        XCTAssertTrue(nothing.isEmpty)
    }

    func testSubscriptionNoteNormalisation() {
        XCTAssertNil(
            AccountStatusRules.subscriptionNote(AccountError(kind: "no_subscription", message: "no_subscription")))
        XCTAssertNil(
            AccountStatusRules.subscriptionNote(AccountError(kind: "x", message: " No  active subscription! ")))
        XCTAssertEqual(AccountStatusRules.normalized("Hello, World — 42"), "hello world 42")
    }
}
