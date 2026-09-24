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
        XCTAssertEqual(PopupScreen.make(phase: .connected, state: full, lastError: nil, serviceIssue: nil), .dashboard(full))
        XCTAssertEqual(PopupScreen.make(phase: .connected, state: empty, lastError: nil, serviceIssue: nil), .empty(empty))
        XCTAssertEqual(
            PopupScreen.make(phase: .connected, state: nil, lastError: .invalidResponse("bad json"), serviceIssue: nil),
            .unreadable(message: "bad json"))
        XCTAssertEqual(
            PopupScreen.make(phase: .connected, state: nil, lastError: .disconnected, serviceIssue: nil), .loading)
    }

    func testSpendAloneStillShowsTheDashboard() throws {
        let hidden = Build.accountJSON(id: "a", hidden: true)
        var state = try Build.state(accounts: [hidden])
        XCTAssertEqual(PopupScreen.make(phase: .connected, state: state, lastError: nil, serviceIssue: nil), .empty(state))
        state = try Build.full()
        XCTAssertTrue(PopupScreen.isRefreshing(try Build.state(accounts: [Build.accountJSON(id: "a", status: "refreshing")])))
        XCTAssertFalse(PopupScreen.isRefreshing(state))
    }

    func testFooterStatusLines() throws {
        let now = try Build.now()
        let full = try Build.full()
        let status = { (screen: PopupScreen) in FooterStatus.make(screen: screen, now: now, formatter: Build.english) }
        XCTAssertEqual(status(.dashboard(full)), FooterStatus(text: "Next update in 3m", isNotice: false, refreshes: true))
        XCTAssertEqual(status(.loading).text, "Connecting…")
        XCTAssertEqual(status(.serviceDown(detail: nil)).text, "Service not running")
        XCTAssertFalse(status(.serviceDown(detail: nil)).refreshes)
        XCTAssertEqual(status(.incompatible(.helperMismatch)).text, "")
        let offline = try Build.state(accounts: [Build.accountJSON(id: "a")], offline: true)
        XCTAssertEqual(status(.dashboard(offline)), FooterStatus(text: "Offline", isNotice: true, refreshes: true))
        let refreshing = try Build.state(accounts: [Build.accountJSON(id: "a", status: "refreshing")])
        XCTAssertEqual(status(.dashboard(refreshing)).text, "Updating…")
        let idle = try Build.state(accounts: [Build.accountJSON(id: "a")])
        XCTAssertEqual(status(.dashboard(idle)).text, "")
    }

    func testOfflineFooterNamesTheLastUpdate() throws {
        var raw = try JSONSerialization.jsonObject(with: Fixture.data("state_full")) as? [String: Any] ?? [:]
        raw["offline"] = true
        let state = try JSONDecoder().decode(DaemonState.self, from: JSONSerialization.data(withJSONObject: raw))
        let line = FooterStatus.make(screen: .dashboard(state), now: try Build.now(), formatter: Build.russian)
        XCTAssertEqual(line, FooterStatus(text: "Нет сети — обновлено в 09:58", isNotice: true, refreshes: true))
    }

    func testDisplayPatchesToggleTheSetting() throws {
        XCTAssertEqual(
            DisplayPatch.toggledValueMode(try Build.display()), ["display": .object(["value_mode": .string("used")])])
        XCTAssertEqual(
            DisplayPatch.toggledResetFormat(try Build.display(resetFormat: "exact")),
            ["display": .object(["reset_format": .string("countdown")])])
    }
}
