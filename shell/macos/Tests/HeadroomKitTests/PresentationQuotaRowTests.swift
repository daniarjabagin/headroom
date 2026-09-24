import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationQuotaRowTests: XCTestCase {
    func testHeadlineFillAndTrailingFollowTheValueMode() throws {
        let window = try Build.window(used: 55, tone: "warning", severity: "close", even: "60", spare: "8.3")
        let left = QuotaRowModel.make(
            window, display: try Build.display(), now: try Build.now(), formatter: Build.english)
        XCTAssertEqual(left.headline, "45% left")
        XCTAssertEqual(left.fill, 0.45, accuracy: 1e-9)
        XCTAssertEqual(left.tone, .warning)
        XCTAssertEqual(left.trailing, "Resets in 2h 0m")
        let used = QuotaRowModel.make(
            window, display: try Build.display(valueMode: "used"), now: try Build.now(), formatter: Build.english)
        XCTAssertEqual(used.headline, "55% used")
        XCTAssertEqual(used.fill, 0.55, accuracy: 1e-9)
    }

    func testFillIsClampedForBoostedPlans() throws {
        let window = try Build.window(used: 130, tone: "critical", severity: "spent")
        let used = QuotaRowModel.make(
            window, display: try Build.display(valueMode: "used"), now: try Build.now(), formatter: Build.english)
        XCTAssertEqual(used.fill, 1)
        let left = QuotaRowModel.make(
            window, display: try Build.display(), now: try Build.now(), formatter: Build.english)
        XCTAssertEqual(left.fill, 0)
        XCTAssertEqual(left.headline, "0% left")
    }

    func testTickPositionMirrorsInLeftModeAndHidesWhenCalmWithoutForecast() throws {
        let calm = try Build.window(used: 20, even: "30")
        XCTAssertEqual(try QuotaRowModel.tick(calm, display: Build.display()) ?? -1, 0.7, accuracy: 1e-9)
        XCTAssertEqual(
            try QuotaRowModel.tick(calm, display: Build.display(valueMode: "used")) ?? -1, 0.3, accuracy: 1e-9)
        XCTAssertNil(try QuotaRowModel.tick(calm, display: Build.display(showForecast: false)))
        let urgent = try Build.window(used: 80, tone: "warning", severity: "close", even: "50")
        XCTAssertNotNil(try QuotaRowModel.tick(urgent, display: Build.display(showForecast: false)))
        XCTAssertNil(try QuotaRowModel.tick(Build.window(), display: Build.display()))
    }

    func testPaceNotes() throws {
        let now = try Build.now()
        let spent = try Build.window(used: 100, tone: "critical", severity: "spent")
        XCTAssertEqual(
            QuotaRowModel.note(spent, now: now, showForecast: true, formatter: Build.english),
            PaceNote(flame: true, text: "Limit reached"))
        let running = try Build.window(
            used: 92, tone: "critical", severity: "running_out", even: "90", runsOut: "\"2026-09-23T10:23:28Z\"")
        XCTAssertEqual(
            QuotaRowModel.note(running, now: now, showForecast: true, formatter: Build.english),
            PaceNote(flame: true, text: "Over pace"))
        XCTAssertEqual(
            QuotaRowModel.note(running, now: now, showForecast: false, formatter: Build.english),
            PaceNote(flame: true, text: "Limit in 23m"))
        let close = try Build.window(used: 55, tone: "warning", severity: "close", spare: "8.4")
        XCTAssertNil(QuotaRowModel.note(close, now: now, showForecast: true, formatter: Build.english))
        XCTAssertEqual(
            QuotaRowModel.note(close, now: now, showForecast: false, formatter: Build.english),
            PaceNote(flame: false, text: "~8% spare"))
        XCTAssertNil(QuotaRowModel.note(try Build.window(), now: now, showForecast: false, formatter: Build.english))
    }

    func testForecasts() throws {
        let now = try Build.now()
        let display = try Build.display()
        let running = try Build.window(
            used: 92, tone: "critical", severity: "running_out", runsOut: "\"2026-09-23T10:23:28Z\"",
            resetsAt: "\"2026-09-23T10:30:00Z\"")
        XCTAssertEqual(
            QuotaRowModel.forecast(running, now: now, display: display, formatter: Build.english),
            "At this pace: runs out in 23m · resets in 30m")
        let pastRunOut = try Build.window(severity: "running_out", runsOut: "\"2026-09-23T09:00:00Z\"")
        XCTAssertEqual(
            QuotaRowModel.forecast(pastRunOut, now: now, display: display, formatter: Build.english),
            "At this pace: runs out any minute")
        let healthy = try Build.window(used: 30, severity: "healthy", projected: "52.5", spare: "47.5")
        XCTAssertEqual(
            QuotaRowModel.forecast(healthy, now: now, display: display, formatter: Build.english),
            "At this pace: ~48% left at reset")
        XCTAssertEqual(
            QuotaRowModel.forecast(
                healthy, now: now, display: try Build.display(valueMode: "used"), formatter: Build.english),
            "At this pace: ~53% used at reset")
        XCTAssertEqual(
            QuotaRowModel.forecast(healthy, now: now, display: display, formatter: Build.russian),
            "При текущем темпе к сбросу останется ~48%")
        XCTAssertNil(
            QuotaRowModel.forecast(
                try Build.window(severity: "untracked"), now: now, display: display, formatter: Build.english))
        let hidden = QuotaRowModel.make(
            healthy, display: try Build.display(showForecast: false), now: now, formatter: Build.english)
        XCTAssertNil(hidden.forecast)
    }

    func testSecondTicksOnlyForCountdownsUnderAnHour() throws {
        let now = try Build.now()
        let soon = try Build.window(resetsAt: "\"2026-09-23T10:40:00Z\"")
        let later = try Build.window(resetsAt: "\"2026-09-23T12:00:00Z\"")
        let unknown = try Build.window(resetsAt: "null")
        XCTAssertTrue(QuotaRowModel.needsSecondTicks(soon, display: try Build.display(), now: now))
        XCTAssertFalse(QuotaRowModel.needsSecondTicks(later, display: try Build.display(), now: now))
        XCTAssertFalse(QuotaRowModel.needsSecondTicks(unknown, display: try Build.display(), now: now))
        XCTAssertFalse(QuotaRowModel.needsSecondTicks(soon, display: try Build.display(resetFormat: "exact"), now: now))
        let row = QuotaRowModel.make(soon, display: try Build.display(), now: now, formatter: Build.english)
        XCTAssertEqual(row.trailing, "Resets in 40m 00s")
        XCTAssertEqual(
            QuotaRowModel.make(unknown, display: try Build.display(), now: now, formatter: Build.english).trailing,
            "Not started")
    }
}
