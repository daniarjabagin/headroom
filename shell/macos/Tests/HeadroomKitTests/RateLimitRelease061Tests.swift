import Foundation
import XCTest

@testable import HeadroomKit

final class RateLimitRelease061Tests: XCTestCase {
    private static let limited = #"{"kind":"rate_limited","message":"usage endpoint rate limited by the provider"}"#

    func testDaemonSnapshotDecodesAsStaleWithTheNextTry() throws {
        let account = try Fixture.decode(Account.self, "account_rate_limited")
        XCTAssertEqual(account.status, .stale)
        XCTAssertEqual(account.error?.kind, "rate_limited")
        XCTAssertEqual(account.refresh?.nextAt, try Fixture.timestamp("2026-09-23T10:05:00Z"))
        XCTAssertTrue(AccountStatusRules.isQuietlyLimited(account))
    }

    func testRateLimitWithDataShowsTheCardWithAQuietNote() throws {
        let section = try onlySection(fixtureState(), formatter: Build.english)
        XCTAssertEqual(section.header.status, .outdated(updatedAt: try Fixture.timestamp("2026-09-23T09:40:00Z")))
        guard case .limits(let limits) = section.body else { return XCTFail("expected limits") }
        XCTAssertEqual(
            limits.notices,
            [
                NoticeModel(
                    id: "rate_limit", kind: .info, title: "Provider is limiting requests · next try 10:05",
                    detail: nil, note: nil, recovery: nil)
            ])
        XCTAssertEqual(limits.windows.map(\.id), ["session"])
    }

    func testTheNoteFollowsLanguageAndTimeFormat() throws {
        let account = try Fixture.decode(Account.self, "account_rate_limited")
        let twelveHour = DisplayFormatter(language: .en, timeZone: Build.utc, hourCycle: .twelveHour)
        XCTAssertEqual(
            RateLimitNote.text(account, formatter: twelveHour),
            "Provider is limiting requests · next try 10:05\u{00A0}AM")
        XCTAssertEqual(
            RateLimitNote.text(account, formatter: Build.russian), "Провайдер ограничил запросы · повтор в 10:05")
    }

    func testRefreshingThroughARateLimitKeepsTheCardQuiet() throws {
        let account = Build.accountJSON(
            id: "a", status: "refreshing", error: Self.limited, updatedAt: #""2026-09-23T09:40:00Z""#,
            recovery: "null")
        let section = try onlySection(Build.state(accounts: [account]), formatter: Build.english)
        guard case .limits(let limits) = section.body else { return XCTFail("expected limits") }
        XCTAssertEqual(limits.notices.map(\.title), ["Provider is limiting requests"])
    }

    func testRateLimitWithoutDataKeepsTheErrorNotice() throws {
        let account = Build.accountJSON(id: "a", status: "error", error: Self.limited, recovery: "null")
        let section = try onlySection(Build.state(accounts: [account]), formatter: Build.english)
        guard case .limits(let limits) = section.body else { return XCTFail("expected limits") }
        XCTAssertEqual(limits.notices.map(\.title), ["Couldn't refresh Codex"])
        XCTAssertEqual(limits.notices.map(\.kind), [.error])
    }

    private func fixtureState() throws -> DaemonState {
        let account = String(decoding: try Fixture.data("account_rate_limited"), as: UTF8.self)
        return try Build.state(accounts: [account])
    }

    private func onlySection(_ state: DaemonState, formatter: DisplayFormatter) throws -> AccountSectionModel {
        try XCTUnwrap(AccountSectionModel.sections(state, formatter: formatter).first)
    }
}
