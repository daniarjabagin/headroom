import Foundation
import XCTest

@testable import HeadroomKit

final class MenuBarContentTests: XCTestCase {
    private static var claude: [String: Any] {
        [
            "account_id": "claude:main", "provider": "claude", "provider_name": "Claude",
            "account_label": "ada@claude.example", "window": "weekly", "window_label": "Weekly", "used_percent": 28.0,
            "remaining_percent": 72.0, "value_percent": 72.0, "tone": "good", "even_pace_percent": 41.5,
            "logo": "claude", "combined": false, "account_count": 1,
        ]
    }

    private static var codex: [String: Any] {
        [
            "account_id": "codex:work", "provider": "codex", "provider_name": "Codex", "account_label": "Work",
            "window": "session", "window_label": "Session", "used_percent": 60.0, "remaining_percent": 40.0,
            "value_percent": 40.0, "tone": "warning", "even_pace_percent": NSNull(), "logo": "codex",
            "combined": false, "account_count": 1,
        ]
    }

    private static var codexGroup: [String: Any] {
        codex.merging(["account_label": NSNull(), "combined": true, "account_count": 2, "tone": "critical"]) {
            _, new in new
        }
    }

    private func state(
        _ items: [[String: Any]], mode: String = "headline", indicator: String = "ring", label: String = "percent",
        valueMode: String = "left", tone: Any = NSNull()
    ) throws -> DaemonState {
        try Build.mutated("state_full") { object in
            object["panel_items"] = items
            object["panel_tone"] = tone
            var display = object["display"] as? [String: Any] ?? [:]
            display["panel_mode"] = mode
            display["panel_indicator"] = indicator
            display["panel_label"] = label
            display["value_mode"] = valueMode
            object["display"] = display
        }
    }

    private func items(_ content: MenuBarContent) throws -> [MenuBarItem] {
        guard case .items(let items) = content else { throw XCTSkip("expected items, got \(content)") }
        return items
    }

    func testHeadlineShowsTheRingAndPercentWithoutLogo() throws {
        let content = MenuBarContent.make(state: try state([Self.claude]), formatter: Build.english)
        let expected = MenuBarItem(
            id: "account/claude:main/weekly", logo: nil, windowLetter: nil, indicator: .ring(fraction: 0.72),
            text: "72%", tone: .good, stale: false, summary: "ada@claude.example · Weekly: 72% left")
        XCTAssertEqual(content, .items([expected]))
    }

    func testHeadlineModeDrawsOnlyTheFirstItem() throws {
        let content = MenuBarContent.make(state: try state([Self.claude, Self.codex]), formatter: Build.english)
        XCTAssertEqual(try items(content).map(\.id), ["account/claude:main/weekly"])
    }

    func testSeveralModeDrawsUpToThreeItemsWithLogoAndLetter() throws {
        let four = [Self.claude, Self.codex, Self.codexGroup, Self.claude]
        let drawn = try items(MenuBarContent.make(state: try state(four, mode: "several"), formatter: Build.english))
        XCTAssertEqual(drawn.count, 3)
        XCTAssertEqual(drawn.map(\.logo), ["claude", "codex", "codex"])
        XCTAssertEqual(drawn.map(\.windowLetter), ["W", "S", "S"])
        XCTAssertEqual(drawn.map(\.text), ["72%", "40%", "40%"])
        XCTAssertEqual(drawn.map(\.tone), [.good, .warning, .critical])
    }

    func testWindowLabelAddsLogoAndLetterInRussian() throws {
        let content = MenuBarContent.make(state: try state([Self.codex], label: "window"), formatter: Build.russian)
        let item = try XCTUnwrap(try items(content).first)
        XCTAssertEqual(item.logo, "codex")
        XCTAssertEqual(item.windowLetter, "С")
        XCTAssertEqual(item.text, "40%")
        XCTAssertEqual(item.summary, "Work · Сессия: Осталось 40%")
    }

    func testRingOnlyHasNoText() throws {
        let content = MenuBarContent.make(state: try state([Self.claude], label: "none"), formatter: Build.english)
        let item = try XCTUnwrap(try items(content).first)
        XCTAssertNil(item.text)
        XCTAssertEqual(item.indicator, .ring(fraction: 0.72))
    }

    func testBarCarriesTheEvenPaceTickForTheDisplayedReading() throws {
        let left = try items(
            MenuBarContent.make(state: try state([Self.claude], indicator: "bar"), formatter: Build.english))
        XCTAssertEqual(left.first?.indicator, .bar(fraction: 0.72, tick: 0.585))
        let used = try items(
            MenuBarContent.make(
                state: try state(
                    [Self.claude.merging(["value_percent": 28.0]) { _, new in new }], indicator: "bar",
                    valueMode: "used"),
                formatter: Build.english))
        XCTAssertEqual(used.first?.indicator, .bar(fraction: 0.28, tick: 0.415))
        let noPace = try items(
            MenuBarContent.make(state: try state([Self.codex], indicator: "bar"), formatter: Build.english))
        XCTAssertEqual(noPace.first?.indicator, .bar(fraction: 0.4, tick: nil))
    }

    func testIndicatorNoneKeepsOnlyText() throws {
        let content = MenuBarContent.make(state: try state([Self.claude], indicator: "none"), formatter: Build.english)
        XCTAssertEqual(try items(content).first?.indicator, MenuBarIndicator.none)
    }

    func testIconModeShowsTheMarkTintedByPanelTone() throws {
        let content = MenuBarContent.make(
            state: try state([], mode: "icon", tone: "warning"), formatter: Build.english)
        XCTAssertEqual(content, .mark(tone: .warning))
        XCTAssertEqual(content.slots, [.mark(tone: .warning)])
    }

    func testNoIndicatorAndNoLabelFallsBackToTheMark() throws {
        let content = MenuBarContent.make(
            state: try state([Self.claude], indicator: "none", label: "none", tone: "good"), formatter: Build.english)
        XCTAssertEqual(content, .mark(tone: .good))
    }

    func testNoItemsOrNoStateShowsTheUntintedMark() throws {
        XCTAssertEqual(MenuBarContent.make(state: try state([]), formatter: Build.english), .glyph)
        XCTAssertEqual(MenuBarContent.make(state: nil, formatter: Build.english), .glyph)
        let empty = try Fixture.decode(DaemonState.self, "state_empty")
        XCTAssertEqual(MenuBarContent.make(state: empty, formatter: Build.english), .glyph)
    }

    func testCombinedGroupIsNamedWithItsAccountCount() throws {
        let content = MenuBarContent.make(state: try state([Self.codexGroup]), formatter: Build.english)
        XCTAssertEqual(try items(content).first?.summary, "Codex ×2 · Session: 40% left")
        let single = Self.codexGroup.merging(["account_count": 1]) { _, new in new }
        let one = MenuBarContent.make(state: try state([single]), formatter: Build.english)
        XCTAssertEqual(try items(one).first?.summary, "Codex · Session: 40% left")
    }

    func testStaleAccountDimsItsItem() throws {
        let hidden = Self.codex.merging(["account_id": "codex:hidden"]) { _, new in new }
        let drawn = try items(
            MenuBarContent.make(state: try state([hidden, Self.codex], mode: "several"), formatter: Build.english))
        XCTAssertEqual(drawn.map(\.stale), [true, false])
    }

    func testLegacyDaemonBuildsTheItemFromTheHeadline() throws {
        let legacy = try Fixture.decode(DaemonState.self, json: StateSamples.legacy)
        let item = try XCTUnwrap(try items(MenuBarContent.make(state: legacy, formatter: Build.english)).first)
        XCTAssertEqual(item.text, "55%")
        XCTAssertEqual(item.indicator, .ring(fraction: 0.55))
        XCTAssertEqual(item.tone, .warning)
    }

    func testOnlyCriticalSlotsPulseAndReducedMotionStopsThem() {
        XCTAssertTrue(MenuBarSlot.mark(tone: .critical).pulses(reducedMotion: false))
        XCTAssertFalse(MenuBarSlot.mark(tone: .critical).pulses(reducedMotion: true))
        XCTAssertFalse(MenuBarSlot.mark(tone: .warning).pulses(reducedMotion: false))
        XCTAssertFalse(MenuBarSlot.mark(tone: nil).pulses(reducedMotion: false))
    }

    func testWindowLetterKeepsShortLabelsAndTakesTheFirstLetterOtherwise() {
        XCTAssertEqual(MenuBarItem.windowLetter("Weekly"), "W")
        XCTAssertEqual(MenuBarItem.windowLetter("session"), "S")
        XCTAssertEqual(MenuBarItem.windowLetter("5h"), "5h")
        XCTAssertEqual(MenuBarItem.windowLetter("5h window"), "5h")
        XCTAssertEqual(MenuBarItem.windowLetter("Неделя"), "Н")
        XCTAssertEqual(MenuBarItem.windowLetter(""), "")
    }
}
