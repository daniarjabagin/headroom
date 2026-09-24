import Foundation
import XCTest

@testable import HeadroomKit

final class FormatterTests: XCTestCase {
    private let utc = TimeZone(identifier: "UTC") ?? .current
    private var english: DisplayFormatter { DisplayFormatter(language: .en, timeZone: utc) }
    private var russian: DisplayFormatter { DisplayFormatter(language: .ru, timeZone: utc) }

    func testPercentReadings() {
        XCTAssertEqual(english.panelPercent(44.5), "45%")
        XCTAssertEqual(english.panelPercent(-3), "0%")
        XCTAssertEqual(english.panelPercent(.nan), "—")
        XCTAssertEqual(english.percentReading(8.4, mode: .left), "8% left")
        XCTAssertEqual(russian.percentReading(92, mode: .used), "Использовано 92%")
        XCTAssertEqual(russian.percentReading(8, mode: .left), "Осталось 8%")
    }

    func testDurations() {
        let cases: [(TimeInterval, Bool, String)] = [
            (0, false, "1m"), (59, false, "1m"), (61, false, "1m"), (3_600, false, "1h 0m"),
            (7_500, false, "2h 5m"), (90_000, false, "1d 1h"), (42, true, "42s"), (0.2, true, "1s"),
            (125, true, "2m 05s"), (4_000, true, "1h 6m"),
        ]
        for (seconds, withSeconds, expected) in cases {
            XCTAssertEqual(english.duration(seconds: seconds, withSeconds: withSeconds), expected, "\(seconds)")
        }
        XCTAssertEqual(russian.duration(seconds: 7_500), "2 ч 5 мин")
    }

    func testResetCountdown() throws {
        let now = try Fixture.timestamp("2026-09-23T10:00:00Z")
        let later = try Fixture.timestamp("2026-09-23T12:05:00Z")
        XCTAssertEqual(english.resetText(resetsAt: later, now: now, format: .countdown), "Resets in 2h 5m")
        XCTAssertEqual(russian.resetText(resetsAt: later, now: now, format: .countdown), "Сброс через 2 ч 5 мин")
        XCTAssertEqual(english.resetText(resetsAt: nil, now: now, format: .countdown), "Not started")
        XCTAssertEqual(english.resetText(resetsAt: now, now: now, format: .countdown), "Reset pending")
        let soon = try Fixture.timestamp("2026-09-23T10:00:30Z")
        XCTAssertEqual(english.resetText(resetsAt: soon, now: now, format: .countdown), "Resets soon")
    }

    func testResetExactUsesCalendarDaysInTimeZone() throws {
        let now = try Fixture.timestamp("2026-09-23T10:00:00Z")
        let cases: [(String, String)] = [
            ("2026-09-23T18:38:00Z", "Resets today at 18:38"),
            ("2026-09-24T00:10:00Z", "Resets tomorrow at 00:10"),
            ("2026-09-26T09:00:00Z", "Resets Sat at 09:00"),
            ("2026-10-02T09:00:00Z", "Resets Oct 2 at 09:00"),
        ]
        for (text, expected) in cases {
            let resetsAt = try Fixture.timestamp(text)
            XCTAssertEqual(english.resetText(resetsAt: resetsAt, now: now, format: .exact), expected)
        }
        let almaty = DisplayFormatter(language: .ru, timeZone: TimeZone(identifier: "Asia/Almaty") ?? utc)
        let lateEvening = try Fixture.timestamp("2026-09-23T19:30:00Z")
        XCTAssertTrue(almaty.resetText(resetsAt: lateEvening, now: now, format: .exact).hasPrefix("Сброс завтра в "))
        let nextWeek = try Fixture.timestamp("2026-10-02T09:00:00Z")
        XCTAssertEqual(russian.resetText(resetsAt: nextWeek, now: now, format: .exact), "Сброс 2 окт. в 09:00")
    }

    func testFooterTexts() throws {
        let now = try Fixture.timestamp("2026-09-23T10:00:00Z")
        XCTAssertEqual(
            english.nextUpdateText(try Fixture.timestamp("2026-09-23T10:00:40Z"), now: now), "Next update in <1m")
        XCTAssertEqual(
            english.nextUpdateText(try Fixture.timestamp("2026-09-23T10:03:00Z"), now: now), "Next update in 3m")
        XCTAssertEqual(russian.agoText(try Fixture.timestamp("2026-09-23T09:00:00Z"), now: now), "1 ч 0 мин назад")
        XCTAssertEqual(english.agoText(now, now: now), "just now")
        XCTAssertEqual(english.clockTime(now.date), "10:00")
    }

    func testMoney() {
        let cases: [(Int64, String, String)] = [
            (0, "$0.00", "$0.00"), (4_999, "$0.00", "$0.00"), (5_000, "$0.01", "$0.01"),
            (12_500_000, "$12.50", "$12.50"), (999_994_999, "$999.99", "$999.99"),
            (1_234_560_000, "$1,234.56", "$1.23K"), (18_420_000_000, "$18,420.00", "$18.4K"),
            (250_000_000_000, "$250,000.00", "$250K"), (-1_500_000, "-$1.50", "-$1.50"),
            (Int64.min, "-$9,223,372,036,854.78", "-$9223B"),
        ]
        for (micros, exact, short) in cases {
            XCTAssertEqual(english.exactUSD(micros: micros), exact, "\(micros)")
            XCTAssertEqual(english.usd(micros: micros), short, "\(micros)")
        }
    }

    func testTokens() {
        XCTAssertEqual(english.exactTokens(35_812_904), "35,812,904")
        XCTAssertEqual(russian.exactTokens(35_812_904), "35\u{00A0}812\u{00A0}904")
        XCTAssertEqual(english.exactTokens(UInt64.max), "18,446,744,073,709,551,615")
        XCTAssertEqual(english.compactTokens(999), "999")
        XCTAssertEqual(english.compactTokens(1_000), "1K")
        XCTAssertEqual(english.compactTokens(35_812_904), "35.8M")
        XCTAssertEqual(english.compactTokens(250_000_000_000), "250B")
        XCTAssertEqual(russian.compactTokens(35_812_904), "35,8\u{00A0}млн")
        XCTAssertEqual(english.compactTokensText(1), "1 token")
        XCTAssertEqual(english.compactTokensText(35_812_904), "35.8M tokens")
        XCTAssertEqual(russian.exactTokensText(21), "21 токен")
        XCTAssertEqual(russian.exactTokensText(22), "22 токена")
        XCTAssertEqual(russian.exactTokensText(12), "12 токенов")
        XCTAssertEqual(russian.compactTokensText(4_812_000), "4,8\u{00A0}млн токенов")
    }

    func testLanguageResolution() {
        XCTAssertEqual(UILanguage.resolve(.system, preferredLanguages: ["ru-KZ", "en"]), .ru)
        XCTAssertEqual(UILanguage.resolve(.system, preferredLanguages: ["en-US"]), .en)
        XCTAssertEqual(UILanguage.resolve(.system, preferredLanguages: []), .en)
        XCTAssertEqual(UILanguage.resolve(.en, preferredLanguages: ["ru"]), .en)
        XCTAssertEqual(UILanguage.resolve(.ru, preferredLanguages: ["en"]), .ru)
    }

    func testEveryUITextIsTranslated() {
        for key in UIText.allCases {
            XCTAssertFalse(UIStrings(language: .ru).text(key).isEmpty)
            XCTAssertNotEqual(UIStrings(language: .ru).text(key), UIStrings(language: .en).text(key))
        }
    }

    func testWindowLabels() {
        XCTAssertEqual(russian.windowLabel(id: "session", label: "Session"), "Сессия")
        XCTAssertEqual(russian.windowLabel(id: "model:opus", label: "Opus"), "Opus")
    }
}
