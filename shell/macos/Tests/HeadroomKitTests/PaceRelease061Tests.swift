import Foundation
import XCTest

@testable import HeadroomKit

final class PaceRelease061Tests: XCTestCase {
    private func pace(_ fields: String) throws -> Pace {
        try Fixture.decode(
            Pace.self,
            json: #"{"severity":"healthy","even_pace_percent":null,"projected_percent":null,"spare_percent":null,"#
                + #""runs_out_at":null\#(fields)}"#)
    }

    private func forecast(
        _ pace: Pace, formatter: DisplayFormatter = Build.english, showForecast: Bool = true
    ) throws -> String? {
        QuotaRowModel.forecast(
            pace, resetsAt: try Fixture.timestamp("2026-09-23T12:00:00Z"), now: try Build.now(),
            display: try Build.display(showForecast: showForecast), formatter: formatter)
    }

    func testBasisAndActiveLeftDecode() throws {
        let paused = try pace(#","basis":"paused","active_left_seconds":10800"#)
        XCTAssertEqual(paused.basis, .paused)
        XCTAssertEqual(paused.activeLeftSeconds, 10_800)
        XCTAssertTrue(paused.isPaused)
        XCTAssertEqual(try pace(#","basis":"recent","active_left_seconds":null"#).basis, .recent)
        XCTAssertEqual(try pace(#","basis":"window""#).basis, .window)
        XCTAssertEqual(try pace(#","basis":"hourly""#).basis, .window)
    }

    func testOlderDaemonsWithoutBasisKeepForecasting() throws {
        let old = try pace("")
        XCTAssertNil(old.basis)
        XCTAssertNil(old.activeLeftSeconds)
        XCTAssertFalse(old.isPaused)
        XCTAssertNil(try pace(#","basis":null"#).basis)
    }

    func testFixtureCarriesTheBasis() throws {
        let state = try Build.full()
        let bases = state.accounts.flatMap(\.windows).map(\.pace.basis)
        XCTAssertEqual(bases, [.recent, .window, .recent])
        XCTAssertTrue(state.display.showBreakdown)
    }

    func testPausedForecastSaysHowLongTheRestLasts() throws {
        let paused = try pace(#","basis":"paused","active_left_seconds":10800"#)
        XCTAssertEqual(try forecast(paused), "Paused · lasts ≈3 h of work")
        XCTAssertEqual(try forecast(paused, formatter: Build.russian), "Пауза · хватит ≈3 ч работы")
    }

    func testPausedWithoutActivePaceSaysPaused() throws {
        let paused = try pace(#","basis":"paused","active_left_seconds":null"#)
        XCTAssertEqual(try forecast(paused), "Paused")
        XCTAssertEqual(try forecast(paused, formatter: Build.russian), "Пауза")
    }

    func testRecentAndWindowKeepTheRunOutLine() throws {
        for basis in ["recent", "window"] {
            let running = try Fixture.decode(
                Pace.self,
                json: #"{"severity":"running_out","even_pace_percent":50,"projected_percent":130,"#
                    + #""spare_percent":null,"runs_out_at":"2026-09-23T11:30:00Z","basis":"\#(basis)"}"#)
            XCTAssertEqual(try forecast(running), "At this pace: runs out in 1h 30m · resets in 2h 0m", basis)
        }
    }

    func testPausedOverPaceNoteDoesNotPromiseALimitSoon() throws {
        let paused = try Fixture.decode(
            Pace.self,
            json: #"{"severity":"running_out","even_pace_percent":50,"projected_percent":130,"spare_percent":null,"#
                + #""runs_out_at":null,"basis":"paused","active_left_seconds":2400}"#)
        let note = QuotaRowModel.note(paused, now: try Build.now(), showForecast: false, formatter: Build.english)
        XCTAssertEqual(note, PaceNote(flame: true, text: "Over pace"))
        XCTAssertEqual(try forecast(paused), "Paused · lasts ≈40 min of work")
    }

    func testApproximateDurationRounding() {
        let english = UIStrings(language: .en)
        let cases: [(UInt64, String)] = [
            (0, "≈1 min"), (29, "≈1 min"), (90, "≈2 min"), (2_400, "≈40 min"), (3_569, "≈59 min"),
            (3_570, "≈1 h"), (8_999, "≈2 h"), (9_000, "≈3 h"), (180_000, "≈50 h"),
        ]
        for (seconds, expected) in cases {
            XCTAssertEqual(PausedForecast.approximate(seconds, strings: english), expected, "\(seconds)")
        }
        XCTAssertEqual(PausedForecast.approximate(2_400, strings: UIStrings(language: .ru)), "≈40 мин")
    }

    func testPooledPausedForecast() throws {
        let state = try Build.combined { object in
            Build.setSessionPace(
                &object,
                [
                    "severity": "close", "even_pace_percent": 110.0, "projected_percent": 188.0,
                    "spare_percent": 12.0, "basis": "paused", "active_left_seconds": 7200,
                ])
        }
        let window = try XCTUnwrap(state.combined.first?.windows.first)
        let text = CombinedRowModel.forecast(
            window, now: try Build.now(), display: try Build.display(), formatter: Build.english)
        XCTAssertEqual(text, "Paused · lasts ≈2 h of work")
    }
}
