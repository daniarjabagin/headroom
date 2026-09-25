import Foundation
import XCTest

@testable import HeadroomKit

final class FormatterRelease06Tests: XCTestCase {
    private let utc = TimeZone(identifier: "UTC") ?? .current
    private var english: DisplayFormatter { DisplayFormatter(language: .en, timeZone: utc) }

    private func formatter(_ language: UILanguage, _ cycle: HourCycle) -> DisplayFormatter {
        DisplayFormatter(language: language, timeZone: utc, hourCycle: cycle)
    }

    func testHourCycleFollowsTheSettingThenTheLocale() {
        let us = Locale(identifier: "en_US")
        let russia = Locale(identifier: "ru_RU")
        XCTAssertEqual(HourCycle.resolve(.auto, locale: us), .twelveHour)
        XCTAssertEqual(HourCycle.resolve(.auto, locale: russia), .twentyFourHour)
        XCTAssertEqual(HourCycle.resolve(.auto, locale: Locale(identifier: "en_GB")), .twentyFourHour)
        XCTAssertEqual(HourCycle.resolve(.twentyFourHour, locale: us), .twentyFourHour)
        XCTAssertEqual(HourCycle.resolve(.twelveHour, locale: russia), .twelveHour)
    }

    func testClockTimesInBothCycles() throws {
        let cases: [(String, String, String)] = [
            ("2026-09-23T00:05:00Z", "00:05", "12:05\u{00A0}AM"),
            ("2026-09-23T09:30:00Z", "09:30", "9:30\u{00A0}AM"),
            ("2026-09-23T12:00:00Z", "12:00", "12:00\u{00A0}PM"),
            ("2026-09-23T18:38:00Z", "18:38", "6:38\u{00A0}PM"),
        ]
        for (text, twentyFour, twelve) in cases {
            let date = try Fixture.timestamp(text).date
            XCTAssertEqual(formatter(.en, .twentyFourHour).clockTime(date), twentyFour)
            XCTAssertEqual(formatter(.en, .twelveHour).clockTime(date), twelve)
        }
    }

    func testExactResetUsesTheHourCycle() throws {
        let now = try Fixture.timestamp("2026-09-23T10:00:00Z")
        let resetsAt = try Fixture.timestamp("2026-09-23T18:38:00Z")
        XCTAssertEqual(
            formatter(.en, .twelveHour).resetText(resetsAt: resetsAt, now: now, format: .exact),
            "Resets today at 6:38\u{00A0}PM")
        XCTAssertEqual(
            formatter(.ru, .twentyFourHour).resetText(resetsAt: resetsAt, now: now, format: .exact),
            "Сброс сегодня в 18:38")
    }

    @MainActor
    func testAppModelFormatterFollowsTheStateTimeFormat() throws {
        let model = AppModel(preferredLanguages: ["en"], timeZone: utc, locale: Locale(identifier: "en_US"))
        XCTAssertEqual(model.formatter.hourCycle, .twelveHour)
        model.apply(.state(try Fixture.decode(DaemonState.self, "state_full")))
        XCTAssertEqual(model.formatter.hourCycle, .twelveHour)
        let forced = try Fixture.text("state_full").replacingOccurrences(
            of: #""time_format": "auto""#, with: #""time_format": "24h""#)
        model.apply(.state(try Fixture.decode(DaemonState.self, json: forced)))
        XCTAssertEqual(model.formatter.hourCycle, .twentyFourHour)
        XCTAssertTrue(model.features.release06)
    }

    func testCostPerMTok() {
        XCTAssertEqual(english.costPerMTok(micros: 2_000_000), "$2.00")
        XCTAssertEqual(english.costPerMTok(micros: 217_163), "$0.22")
        XCTAssertEqual(english.costPerMTok(micros: 786_271), "$0.79")
        XCTAssertEqual(english.costPerMTok(micros: 15_004_999), "$15.00")
        XCTAssertEqual(english.costPerMTok(micros: 4_999), "<$0.01")
        XCTAssertEqual(english.costPerMTok(micros: 5_000), "$0.01")
        XCTAssertEqual(english.costPerMTok(micros: 0), "$0.00")
        XCTAssertEqual(english.costPerMTok(micros: nil), "—")
    }

    func testProjectNamesUseAMiddleEllipsis() {
        let path = "~/code/clients/acme/very-long/headroom"
        XCTAssertEqual(DisplayFormatter.middleEllipsis(path, maxCharacters: 40), path)
        XCTAssertEqual(DisplayFormatter.middleEllipsis(path, maxCharacters: 24), "~/code/clients…/headroom")
        XCTAssertEqual(DisplayFormatter.middleEllipsis(path, maxCharacters: 24).count, 24)
        XCTAssertEqual(DisplayFormatter.middleEllipsis(path, maxCharacters: 10), "~/cod…room")
        XCTAssertEqual(DisplayFormatter.middleEllipsis("abcdefghij", maxCharacters: 5), "ab…ij")
        XCTAssertEqual(DisplayFormatter.middleEllipsis("~/a/b/", maxCharacters: 5), "~…/b/")
        XCTAssertEqual(DisplayFormatter.middleEllipsis("abc", maxCharacters: 1), "…")
        XCTAssertEqual(DisplayFormatter.middleEllipsis("abc", maxCharacters: 0), "")
        XCTAssertEqual(english.projectName(nil, maxCharacters: 30), "No project")
        XCTAssertEqual(DisplayFormatter(language: .ru).projectName(nil, maxCharacters: 30), "Без проекта")
        XCTAssertEqual(english.projectName("~/code/headroom", maxCharacters: 30), "~/code/headroom")
    }
}
