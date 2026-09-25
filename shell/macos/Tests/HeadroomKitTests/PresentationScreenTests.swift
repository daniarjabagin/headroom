import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationScreenTests: XCTestCase {
    func testScreenFollowsConnectionPhaseAndState() throws {
        let full = try Build.full()
        let empty = try Fixture.decode(DaemonState.self, "state_empty")
        XCTAssertEqual(PopupScreen.make(phase: .starting, state: nil, lastError: nil, serviceIssue: nil), .loading)
        XCTAssertEqual(
            PopupScreen.make(phase: .disconnected, state: full, lastError: nil, serviceIssue: "exited"),
            .serviceDown(detail: "exited"))
        XCTAssertEqual(
            PopupScreen.make(phase: .incompatible(.schemaMismatch), state: full, lastError: nil, serviceIssue: nil),
            .incompatible(.schemaMismatch))
        XCTAssertEqual(
            PopupScreen.make(phase: .connected, state: full, lastError: nil, serviceIssue: nil), .dashboard(full))
        XCTAssertEqual(
            PopupScreen.make(phase: .connected, state: empty, lastError: nil, serviceIssue: nil), .empty(empty))
        XCTAssertEqual(
            PopupScreen.make(phase: .connected, state: nil, lastError: .invalidResponse("bad json"), serviceIssue: nil),
            .unreadable(message: "bad json"))
        XCTAssertEqual(
            PopupScreen.make(phase: .connected, state: nil, lastError: .disconnected, serviceIssue: nil), .loading)
    }

    func testSpendAloneStillShowsTheDashboard() throws {
        let hidden = Build.accountJSON(id: "a", hidden: true)
        var state = try Build.state(accounts: [hidden])
        XCTAssertEqual(
            PopupScreen.make(phase: .connected, state: state, lastError: nil, serviceIssue: nil), .empty(state))
        state = try Build.full()
        XCTAssertTrue(
            PopupScreen.isRefreshing(try Build.state(accounts: [Build.accountJSON(id: "a", status: "refreshing")])))
        XCTAssertFalse(PopupScreen.isRefreshing(state))
    }

    private func footer(_ screen: PopupScreen, formatter: DisplayFormatter = Build.english) throws -> FooterModel {
        FooterModel.make(screen: screen, now: try Build.now(), formatter: formatter, version: "Headroom 0.6.0")
    }

    func testFooterIdleAndPlainLines() throws {
        let full = try footer(.dashboard(try Build.full()))
        XCTAssertEqual(
            full,
            FooterModel(
                primary: "Updated 2m ago", secondary: "Next update in 3m", kind: .plain, tip: nil, refreshes: true))
        XCTAssertEqual(try footer(.loading), FooterModel.plain("Headroom 0.6.0", "Connecting…"))
        XCTAssertEqual(try footer(.serviceDown(detail: nil)).secondary, "Service not running")
        XCTAssertFalse(try footer(.serviceDown(detail: nil)).refreshes)
        XCTAssertEqual(try footer(.incompatible(.helperMismatch)).secondary, "")
        let refreshing = try Build.state(accounts: [Build.accountJSON(id: "a", status: "refreshing")])
        XCTAssertEqual(try footer(.dashboard(refreshing)).secondary, "Updating…")
        let idle = try Build.state(accounts: [Build.accountJSON(id: "a")])
        XCTAssertEqual(try footer(.dashboard(idle)).primary, "Headroom 0.6.0")
        XCTAssertEqual(try footer(.dashboard(idle)).secondary, "")
    }

    func testFooterUsesClockTimesInExactMode() throws {
        let state = try Build.mutated("state_full") { object in
            var display = object["display"] as? [String: Any] ?? [:]
            display["reset_format"] = "exact"
            display["time_format"] = "12h"
            object["display"] = display
        }
        let formatter = DisplayFormatter(language: .en, timeZone: Build.utc, hourCycle: .twelveHour)
        let model = try footer(.dashboard(state), formatter: formatter)
        XCTAssertEqual(model.primary, "Updated 9:58\u{00A0}AM")
        XCTAssertEqual(model.secondary, "Next update at 10:03\u{00A0}AM")
    }

    func testOfflineFooterIsStale() throws {
        let state = try Build.mutated("state_full") { $0["offline"] = true }
        let english = try footer(.dashboard(state))
        XCTAssertEqual(english.kind, .stale)
        XCTAssertEqual(english.primary, "Outdated · updated 2m ago")
        XCTAssertEqual(english.secondary, "Offline — retrying in 3m")
        let russian = try footer(.dashboard(state), formatter: Build.russian)
        XCTAssertEqual(russian.primary, "Устарело · обновлено 2 мин назад")
        let unknown = try Build.state(accounts: [Build.accountJSON(id: "a")], offline: true)
        XCTAssertEqual(try footer(.dashboard(unknown)).primary, "Outdated")
        XCTAssertEqual(try footer(.dashboard(unknown)).secondary, "Offline")
    }

    func testStaleAccountMakesTheFooterStale() throws {
        let stale = Build.accountJSON(id: "a", status: "stale", updatedAt: "\"2026-09-23T09:48:00Z\"")
        let model = try footer(.dashboard(try Build.state(accounts: [stale])))
        XCTAssertEqual(model.kind, .stale)
        XCTAssertEqual(model.primary, "Outdated · updated 12m ago")
    }

    func testLiveFooterExplainsAdaptiveRefresh() throws {
        let state = try Build.mutated("state_full") { object in
            guard var accounts = object["accounts"] as? [[String: Any]] else { return }
            accounts[0]["refresh"] = [
                "mode": "live", "interval_secs": 60, "next_at": "2026-09-23T10:01:00Z", "reason": "activity",
            ]
            object["accounts"] = accounts
        }
        let model = try footer(.dashboard(state))
        XCTAssertEqual(model.kind, .live)
        XCTAssertEqual(model.secondary, "Live — every minute while Codex is active")
        XCTAssertEqual(
            model.tip,
            "Codex is writing local logs, so it is checked every minute. Back to every 5 minutes after 10 min without activity."
        )
    }

    func testDisplayTogglesFlipTheSetting() throws {
        XCTAssertEqual(DisplayToggle.valueMode(try Build.display()), .valueMode(.used))
        XCTAssertEqual(DisplayToggle.valueMode(try Build.display(valueMode: "used")), .valueMode(.left))
        XCTAssertEqual(DisplayToggle.resetFormat(try Build.display()), .resetFormat(.exact))
        XCTAssertEqual(DisplayToggle.resetFormat(try Build.display(resetFormat: "exact")), .resetFormat(.countdown))
    }
}
