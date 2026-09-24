import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationCombinedTests: XCTestCase {
    private func combined() throws -> DaemonState {
        try Fixture.decode(DaemonState.self, "state_combined")
    }

    private func window(_ id: String) throws -> CombinedWindow {
        try XCTUnwrap(try combined().combined.first?.windows.first { $0.id == id })
    }

    func testDecodesCombinedGroupsAndHeadline() throws {
        let state = try combined()
        XCTAssertTrue(state.display.combineAccounts)
        let group = try XCTUnwrap(state.combined.first)
        XCTAssertEqual(group.accountIDs, ["codex:work", "codex:personal"])
        XCTAssertEqual(group.accounts.map(\.plan), ["Pro", "Plus"])
        XCTAssertEqual(group.windows.first?.capacityPercent, 200)
        XCTAssertEqual(group.windows.first?.segments.map(\.id), ["codex:work", "codex:personal"])
        let headline = try XCTUnwrap(state.headline)
        XCTAssertTrue(headline.combined)
        XCTAssertEqual(headline.accountCount, 2)
        XCTAssertNil(headline.accountLabel)
    }

    func testOlderDaemonsDecodeWithoutCombinedFields() throws {
        let state = try Build.full()
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
        let ordered = [state.accounts[1], state.accounts[2], state.accounts[0]]
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
        XCTAssertTrue(limits.notices.isEmpty)
        let russian = try XCTUnwrap(AccountSectionModel.sections(try combined(), formatter: Build.russian).first)
        XCTAssertEqual(russian.header.accountCount, "2 аккаунта")
    }

    func testHiddenMemberLeavesTheCardButKeepsTheDaemonCount() throws {
        var object = try XCTUnwrap(
            JSONSerialization.jsonObject(with: Fixture.data("state_combined")) as? [String: Any])
        let accounts = try XCTUnwrap(object["accounts"] as? [[String: Any]])
        object["accounts"] = accounts.map { account in
            var copy = account
            if copy["id"] as? String == "codex:work" { copy["hidden"] = true }
            return copy
        }
        let state = try JSONDecoder().decode(DaemonState.self, from: JSONSerialization.data(withJSONObject: object))
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
        XCTAssertEqual(notices.first?.retryAccountID, "b")
    }

    func testCombinedRowReadsLeftOfCapacity() throws {
        let row = CombinedRowModel.make(
            try window("session"), display: try Build.display(), now: try Build.now(), formatter: Build.english)
        XCTAssertEqual(row.label, "Session")
        XCTAssertEqual(row.headline, "145% left of 200%")
        XCTAssertEqual(row.segments.map(\.fill), [0.95, 0.5])
        XCTAssertEqual(row.segments.map(\.tone), [.good, .good])
        XCTAssertEqual(row.trailing, "Resets in 1h 0m")
        XCTAssertEqual(row.forecast, "At this pace: ~70% left at reset")
        XCTAssertEqual(
            row.tip,
            .lines(title: "Work 95% · Personal 50%", lines: ["Work · Resets in 2h 0m", "Personal · Resets in 1h 0m"]))
    }

    func testCombinedRowInUsedModeAndRussian() throws {
        let display = try Build.display(valueMode: "used")
        let used = CombinedRowModel.make(
            try window("weekly"), display: display, now: try Build.now(), formatter: Build.english)
        XCTAssertEqual(used.headline, "110% used of 200%")
        XCTAssertEqual(used.segments.map(\.fill), [0.3, 0.8])
        XCTAssertEqual(used.segments.map(\.tone), [.good, .warning])
        XCTAssertEqual(used.note, PaceNote(flame: true, text: "Over pace"))
        XCTAssertEqual(used.forecast, "At this pace: runs out in 1d 12h · resets in 2d 0h")
        let russian = CombinedRowModel.make(
            try window("session"), display: try Build.display(), now: try Build.now(), formatter: Build.russian)
        XCTAssertEqual(russian.headline, "Осталось 145% из 200%")
        XCTAssertEqual(russian.label, "Сессия")
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

    func testMenuBarNamesTheCombinedGroup() throws {
        let json = try Fixture.text("state_combined")
            .replacingOccurrences(of: #""panel_label": "percent""#, with: #""panel_label": "window""#)
        let state = try Fixture.decode(DaemonState.self, json: json)
        XCTAssertEqual(
            MenuBarContent.make(state: state, formatter: Build.english),
            .reading(text: "Codex ×2 · Weekly", fraction: 0.45))
        XCTAssertEqual(
            MenuBarContent.make(state: state, formatter: Build.russian),
            .reading(text: "Codex ×2 · Неделя", fraction: 0.45))
        XCTAssertEqual(
            MenuBarContent.make(state: try combined(), formatter: Build.english), .reading(text: "90%", fraction: 0.45))
        XCTAssertEqual(MenuBarContent.subject(try XCTUnwrap(state.headline)), "Codex ×2")
        XCTAssertEqual(MenuBarContent.subject(try XCTUnwrap(try Build.full().headline)), "ada@claude.example")
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
