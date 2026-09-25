import XCTest

@testable import HeadroomKit

final class PanelLimitChoicesTests: XCTestCase {
    private func accounts() throws -> [Account] {
        let windows = [Build.windowJSON(id: "session"), Build.windowJSON(id: "weekly")]
        return try Build.state(accounts: [
            Build.accountJSON(id: "claude:a", provider: "claude", windows: windows),
            Build.accountJSON(id: "codex:b", provider: "codex", windows: [Build.windowJSON(id: "weekly")]),
            Build.accountJSON(id: "codex:hidden", provider: "codex", windows: windows, hidden: true),
        ]).accounts
    }

    private func limit(_ account: String, _ window: String) -> PanelLimit {
        PanelLimit(accountID: account, window: window)
    }

    func testOptionsListVisibleWindowsAndKeepUnavailablePins() throws {
        let options = PanelLimitChoices.options(
            accounts: try accounts(), current: [limit("gone:x", "session")], formatter: Build.english)
        XCTAssertEqual(
            options.map(\.label),
            ["Claude Session", "Claude Weekly", "Codex Weekly", "Pinned limit (not available now)"])
        XCTAssertEqual(options.map(\.available), [true, true, true, false])
        XCTAssertEqual(options.last?.limit, limit("gone:x", "session"))
    }

    func testToggleAddsInOrderUpToThreeAndRemoves() {
        let one = limit("a", "session")
        let two = limit("b", "weekly")
        let three = limit("c", "session")
        let four = limit("d", "session")
        var current = PanelLimitChoices.toggled(two, in: [])
        current = PanelLimitChoices.toggled(one, in: current)
        current = PanelLimitChoices.toggled(three, in: current)
        XCTAssertEqual(current, [two, one, three])
        XCTAssertEqual(PanelLimitChoices.toggled(four, in: current), current)
        XCTAssertFalse(PanelLimitChoices.canAdd(four, to: current))
        XCTAssertTrue(PanelLimitChoices.canAdd(one, to: current))
        XCTAssertEqual(PanelLimitChoices.toggled(one, in: current), [two, three])
    }

    func testSummaryFollowsTheStoredOrder() throws {
        let current = [limit("codex:b", "weekly"), limit("claude:a", "session")]
        let options = PanelLimitChoices.options(accounts: try accounts(), current: current, formatter: Build.english)
        let strings = Build.english.strings
        XCTAssertEqual(
            PanelLimitChoices.summary(current, options: options, strings: strings), "Codex Weekly, Claude Session")
        XCTAssertEqual(PanelLimitChoices.summary([], options: options, strings: strings), "Most critical")
        XCTAssertEqual(
            PanelLimitChoices.summary([], options: options, strings: Build.russian.strings), "Самые критичные")
    }
}
