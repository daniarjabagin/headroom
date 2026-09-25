import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationCombinedTests: XCTestCase {
    private func combined() throws -> DaemonState {
        try Fixture.decode(DaemonState.self, "state_combined")
    }

    private func window(_ id: String, in state: DaemonState? = nil) throws -> CombinedWindow {
        try XCTUnwrap(try (state ?? combined()).combined.first?.windows.first { $0.id == id })
    }

    private func members(_ state: DaemonState) -> [Account] {
        state.accounts.filter { $0.provider == "codex" && !$0.hidden }
    }

    private func pooledRow(
        _ state: DaemonState, display: DisplaySettings, formatter: DisplayFormatter = Build.english
    ) throws -> CombinedRowModel {
        let row = CombinedLimitRow.make(
            try window("session", in: state), members: members(state), display: display, now: try Build.now(),
            formatter: formatter)
        guard case .pooled(let model) = row else { return try XCTUnwrap(nil, "expected a pooled row, got \(row)") }
        return model
    }

    func testDecodesTheDaemonCombinedSnapshot() throws {
        let state = try combined()
        XCTAssertTrue(state.display.combineAccounts)
        let group = try XCTUnwrap(state.combined.first)
        XCTAssertEqual(group.accountIDs, ["codex:work", "codex:personal"])
        XCTAssertEqual(group.accounts.map(\.plan), ["Pro", "Plus"])
        XCTAssertEqual(group.windows.map(\.capacityPercent), [200, 100])
        XCTAssertEqual(group.windows.first?.segments.map(\.id), ["codex:work", "codex:personal"])
        XCTAssertEqual(group.windows.last?.segments.map(\.id), ["codex:personal"])
        XCTAssertNil(group.windows.first?.pace.runsOutAt)
        let headline = try XCTUnwrap(state.headline)
        XCTAssertFalse(headline.combined)
        XCTAssertEqual(headline.accountCount, 1)
    }

    func testOlderDaemonsDecodeWithoutCombinedFields() throws {
        let state = try Build.mutated("state_full") { object in
            object["combined"] = nil
            var display = object["display"] as? [String: Any] ?? [:]
            display["combine_accounts"] = nil
            object["display"] = display
            var headline = object["headline"] as? [String: Any] ?? [:]
            headline["combined"] = nil
            headline["account_count"] = nil
            object["headline"] = headline
        }
        XCTAssertEqual(state.combined, [])
        XCTAssertFalse(state.display.combineAccounts)
        XCTAssertEqual(state.headline?.combined, false)
        XCTAssertNil(state.headline?.accountCount)
    }

    func testCombinedCardReplacesItsMembersAtTheFirstMemberPosition() throws {
        let state = try combined()
        let sections = AccountSectionModel.sections(state, formatter: Build.english)
        XCTAssertEqual(sections.map(\.id), ["combined:codex", "claude:main"])
        XCTAssertEqual(sections.map(\.memberIDs), [["codex:work", "codex:personal"], ["claude:main"]])
        let ordered = [state.accounts[1], state.accounts[3], state.accounts[0], state.accounts[2]]
        let reordered = AccountSectionModel.sections(state, ordered: ordered, formatter: Build.english)
        XCTAssertEqual(reordered.map(\.id), ["claude:main", "combined:codex"])
        XCTAssertEqual(reordered.last?.memberIDs, ["codex:personal", "codex:work"])
    }

    func testCombinedHeaderCountsAccountsAndListsPlans() throws {
        let english = try XCTUnwrap(AccountSectionModel.sections(try combined(), formatter: Build.english).first)
        XCTAssertEqual(
            english.header,
            AccountHeaderModel(
                provider: "codex", title: "Codex", plan: "Pro · Plus", status: nil, accountCount: "2 accounts"))
        guard case .combined(let limits) = english.body else { return XCTFail("expected a combined card") }
        XCTAssertEqual(limits.windows.map(\.id), ["session", "weekly"])
        XCTAssertEqual(limits.members.map(\.id), ["codex:work", "codex:personal"])
        XCTAssertEqual(limits.notices.map(\.id), ["codex:work:notice:0"])
        let russian = try XCTUnwrap(AccountSectionModel.sections(try combined(), formatter: Build.russian).first)
        XCTAssertEqual(russian.header.accountCount, "2 аккаунта")
    }

    func testHiddenMemberLeavesTheCardButKeepsTheDaemonCount() throws {
        let state = try Build.combined { object in
            let accounts = object["accounts"] as? [[String: Any]] ?? []
            object["accounts"] = accounts.map { account in
                var copy = account
                if copy["id"] as? String == "codex:work" { copy["hidden"] = true }
                return copy
            }
        }
        let sections = AccountSectionModel.sections(state, formatter: Build.english)
        XCTAssertEqual(sections.map(\.id), ["claude:main", "combined:codex"])
        XCTAssertEqual(sections.last?.memberIDs, ["codex:personal"])
    }

    func testGroupStatusPrefersRefreshingThenFailures() throws {
        let error = #"{"kind":"network","message":"timeout"}"#
        let state = try Build.state(accounts: [
            Build.accountJSON(id: "a", label: #""A""#, status: "stale", updatedAt: #""2026-09-23T09:00:00Z""#),
            Build.accountJSON(id: "b", label: #""B""#, status: "error", error: error),
        ])
        let statuses = AccountSectionModel.groupStatus(state.accounts, offline: false)
        XCTAssertEqual(statuses, .failed(message: "timeout"))
        let notices = AccountSectionModel.memberNotices(state.accounts, offline: false, strings: Build.english.strings)
        XCTAssertEqual(notices.map(\.title), ["Couldn't refresh Codex: B"])
        XCTAssertEqual(notices.map(\.id), ["b:error"])
        XCTAssertEqual(notices.first?.recovery?.accountID, "b")
    }

    func testPooledRowReadsAndForecastsOnTheCapacityScale() throws {
        let row = try pooledRow(try combined(), display: try Build.display())
        XCTAssertEqual(row.label, "Session")
        XCTAssertEqual(row.headline, "125% left of 200%")
        XCTAssertEqual(row.segments.map(\.fill), [0.45, 0.8])
        XCTAssertEqual(row.segments.map(\.tone), [.warning, .good])
        XCTAssertEqual(row.trailing, "Resets in 2h 0m")
        XCTAssertNil(row.note)
        XCTAssertEqual(row.forecast, "At this pace: ~68% of 200% left at reset")
        XCTAssertEqual(
            row.tip,
            .lines(
                title: "Work 45% · Personal 80%",
                lines: ["Work · Resets in 2h 0m", "Personal · Resets in 2h 30m"]))
        let used = try pooledRow(try combined(), display: try Build.display(valueMode: "used"))
        XCTAssertEqual(used.headline, "75% used of 200%")
        XCTAssertEqual(used.forecast, "At this pace: ~132% of 200% used at reset")
        let russian = try pooledRow(try combined(), display: try Build.display(), formatter: Build.russian)
        XCTAssertEqual(russian.headline, "Осталось 125% из 200%")
        XCTAssertEqual(russian.label, "Сессия")
        XCTAssertEqual(russian.forecast, "При текущем темпе к сбросу останется ~68% из 200%")
    }

    func testPooledSegmentsTickAtEachAccountsOwnPace() throws {
        let left = try pooledRow(try combined(), display: try Build.display())
        XCTAssertEqual(left.segments.map(\.tick), [0.4, 0.5])
        let used = try pooledRow(try combined(), display: try Build.display(valueMode: "used"))
        XCTAssertEqual(used.segments.map(\.tick), [0.6, 0.5])
        let quiet = try pooledRow(try combined(), display: try Build.display(showForecast: false))
        XCTAssertEqual(quiet.segments.map(\.tick), [0.4, nil])
    }

    func testPooledRunningOutHasNoRunOutTime() throws {
        let state = try Build.combined { object in
            Build.setSessionPace(
                &object,
                ["severity": "running_out", "even_pace_percent": 110.0, "projected_percent": 212.0])
        }
        let forecasting = try pooledRow(state, display: try Build.display())
        XCTAssertEqual(forecasting.note, PaceNote(flame: true, text: "Over pace"))
        XCTAssertEqual(forecasting.forecast, "At this pace: runs out before reset")
        let quiet = try pooledRow(state, display: try Build.display(showForecast: false))
        XCTAssertEqual(quiet.note, PaceNote(flame: true, text: "Over pace"))
        XCTAssertNil(quiet.forecast)
        let russian = try pooledRow(state, display: try Build.display(), formatter: Build.russian)
        XCTAssertEqual(russian.forecast, "При текущем темпе закончится до сброса")
    }

    func testPooledCloseSpareStaysOnTheCapacityScale() throws {
        let state = try Build.combined { object in
            Build.setSessionPace(
                &object,
                [
                    "severity": "close", "even_pace_percent": 110.0, "projected_percent": 188.0,
                    "spare_percent": 12.0,
                ])
        }
        XCTAssertEqual(
            try pooledRow(state, display: try Build.display(showForecast: false)).note,
            PaceNote(flame: false, text: "~12% spare"))
        XCTAssertEqual(
            try pooledRow(state, display: try Build.display()).forecast, "At this pace: ~12% of 200% left at reset")
        XCTAssertEqual(
            try pooledRow(state, display: try Build.display(valueMode: "used")).forecast,
            "At this pace: ~188% of 200% used at reset")
    }

    func testSingleAccountWindowRendersLikeANormalRow() throws {
        let state = try combined()
        let display = try Build.display()
        let row = CombinedLimitRow.make(
            try window("weekly"), members: members(state), display: display, now: try Build.now(),
            formatter: Build.english)
        let personal = try XCTUnwrap(state.accounts.first { $0.id == "codex:personal" }?.windows.last)
        let expected = QuotaRowModel.make(personal, display: display, now: try Build.now(), formatter: Build.english)
        XCTAssertEqual(row, .single(expected))
        XCTAssertEqual(row.id, "weekly")
        XCTAssertEqual(expected.headline, "40% left")
        XCTAssertEqual(expected.forecast, "At this pace: ~16% left at reset")
        XCTAssertFalse(CombinedLimitRow.isPooled(try window("weekly")))
        XCTAssertTrue(CombinedLimitRow.isPooled(try window("session")))
    }

    func testSegmentSpansShareTheWidthWithGaps() {
        XCTAssertEqual(
            SegmentedMeter.spans(count: 2, width: 100),
            [MeterSpan(offset: 0, width: 49), MeterSpan(offset: 51, width: 49)])
        XCTAssertEqual(SegmentedMeter.spans(count: 1, width: 80), [MeterSpan(offset: 0, width: 80)])
        let three = SegmentedMeter.spans(count: 3, width: 100)
        XCTAssertEqual(three.last.map { $0.offset + $0.width } ?? 0, 100, accuracy: 1e-9)
        XCTAssertEqual(SegmentedMeter.spans(count: 0, width: 100), [])
        XCTAssertEqual(SegmentedMeter.spans(count: 2, width: 0), [])
    }

    func testSegmentFillKeepsAVisibleMinimumAndClamps() {
        XCTAssertEqual(SegmentedMeter.fillWidth(0, span: 50, minimum: 5), 0)
        XCTAssertEqual(SegmentedMeter.fillWidth(0.01, span: 50, minimum: 5), 5)
        XCTAssertEqual(SegmentedMeter.fillWidth(0.5, span: 50, minimum: 5), 25)
        XCTAssertEqual(SegmentedMeter.fillWidth(1.3, span: 50, minimum: 5), 50)
        XCTAssertEqual(SegmentedMeter.fillWidth(.nan, span: 50, minimum: 5), 0)
        XCTAssertEqual(SegmentedMeter.fillWidth(0.1, span: 3, minimum: 5), 3)
    }

    func testSegmentTickStaysInsideItsSpan() {
        XCTAssertEqual(SegmentedMeter.tickOffset(0.5, span: 50, tickWidth: 2), 24)
        XCTAssertEqual(SegmentedMeter.tickOffset(0, span: 50, tickWidth: 2), 0)
        XCTAssertEqual(SegmentedMeter.tickOffset(1, span: 50, tickWidth: 2), 48)
        XCTAssertEqual(SegmentedMeter.tickOffset(1.4, span: 50, tickWidth: 2), 48)
        XCTAssertEqual(SegmentedMeter.tickOffset(.nan, span: 50, tickWidth: 2), 0)
        XCTAssertEqual(SegmentedMeter.tickOffset(0.5, span: 1, tickWidth: 2), 0)
    }

    func testMenuBarNamesTheCombinedGroup() throws {
        let state = try Build.combined(headline: Build.combinedHeadline(window: "session", count: 2), label: "window")
        XCTAssertEqual(
            MenuBarContent.make(state: state, formatter: Build.english),
            .reading(text: "Codex ×2 · Session", fraction: 0.625))
        XCTAssertEqual(
            MenuBarContent.make(state: state, formatter: Build.russian),
            .reading(text: "Codex ×2 · Сессия", fraction: 0.625))
        let percent = try Build.combined(headline: Build.combinedHeadline(window: "session", count: 2))
        XCTAssertEqual(
            MenuBarContent.make(state: percent, formatter: Build.english), .reading(text: "63%", fraction: 0.625))
        XCTAssertEqual(MenuBarContent.subject(try XCTUnwrap(state.headline)), "Codex ×2")
        XCTAssertEqual(MenuBarContent.subject(try XCTUnwrap(try combined().headline)), "ada@claude.example")
    }

    func testSingleSegmentCombinedHeadlineNamesTheProviderOnly() throws {
        let state = try Build.combined(headline: Build.combinedHeadline(window: "weekly", count: 1), label: "window")
        XCTAssertEqual(
            MenuBarContent.make(state: state, formatter: Build.english),
            .reading(text: "Codex · Weekly", fraction: 0.625))
    }

    func testAccountCountPlurals() {
        let russian = Build.russian.strings
        XCTAssertEqual(russian.fill(.accounts, count: 1), "1 аккаунт")
        XCTAssertEqual(russian.fill(.accounts, count: 3), "3 аккаунта")
        XCTAssertEqual(russian.fill(.accounts, count: 5), "5 аккаунтов")
        XCTAssertEqual(russian.fill(.accounts, count: 21), "21 аккаунт")
        XCTAssertEqual(Build.english.strings.fill(.accounts, count: 1), "1 account")
        XCTAssertEqual(Build.english.strings.fill(.accounts, count: 2), "2 accounts")
    }
}
