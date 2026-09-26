import Foundation
import XCTest

@testable import HeadroomKit

final class MotionRelease061Tests: XCTestCase {
    private let ring = PopupLayout.normal.donutSize
    private var hole: Double { ring * DonutGeometry.holeRatio }

    func testShortAmountKeepsItsSize() {
        let fit = RingLabelFit.make(text: "$18.42", ringSize: ring, amountSize: 13, captionSize: nil)
        XCTAssertEqual(fit.fontSize, 13)
        XCTAssertLessThan(fit.width, hole)
    }

    func testLongTokenAmountShrinksToTheHole() {
        let fit = RingLabelFit.make(text: "35,8\u{00A0}млн", ringSize: ring, amountSize: 13, captionSize: 9)
        XCTAssertLessThan(fit.fontSize, 13)
        XCTAssertGreaterThanOrEqual(fit.fontSize, 13 * RingLabelFit.minimumScale)
        let drawn = RingLabelFit.estimatedWidth("35,8\u{00A0}млн", fontSize: fit.fontSize)
        XCTAssertLessThanOrEqual(drawn, fit.width + 1e-9)
    }

    func testCaptionNarrowsTheLabel() {
        let alone = RingLabelFit.make(text: "196M", ringSize: ring, amountSize: 13, captionSize: nil)
        let captioned = RingLabelFit.make(text: "196M", ringSize: ring, amountSize: 13, captionSize: 9)
        XCTAssertLessThan(captioned.width, alone.width)
    }

    func testHugeTextStopsAtTheMinimumScale() {
        let fit = RingLabelFit.make(text: "$1,234,567,890.00", ringSize: 64, amountSize: 12, captionSize: 9)
        XCTAssertEqual(fit.fontSize, 12 * RingLabelFit.minimumScale, accuracy: 1e-9)
    }

    func testChordNeverGoesNegative() {
        XCTAssertEqual(RingLabelFit.chord(radius: 10, height: 0), 20, accuracy: 1e-9)
        XCTAssertEqual(RingLabelFit.chord(radius: 5, height: 40), 0)
    }

    func testSheenRunsOnlyOnVisibleFillsWithMotion() {
        XCTAssertTrue(MeterSheen.runs(fill: 0.5, reducedMotion: false))
        XCTAssertTrue(MeterSheen.runs(fill: 0.03, reducedMotion: false))
        XCTAssertFalse(MeterSheen.runs(fill: 0.02, reducedMotion: false))
        XCTAssertFalse(MeterSheen.runs(fill: 0.5, reducedMotion: true))
        XCTAssertFalse(MeterSheen.runs(fill: .nan, reducedMotion: false))
    }

    func testSheenSweepsThenRests() throws {
        XCTAssertEqual(MeterSheen.progress(elapsed: 0), 0)
        XCTAssertEqual(try XCTUnwrap(MeterSheen.progress(elapsed: 0.8)), 0.5, accuracy: 1e-9)
        XCTAssertNil(MeterSheen.progress(elapsed: 1.6))
        XCTAssertNil(MeterSheen.progress(elapsed: 4.0))
        XCTAssertEqual(try XCTUnwrap(MeterSheen.progress(elapsed: 5.1 + 0.8)), 0.5, accuracy: 1e-9)
        XCTAssertNil(MeterSheen.progress(elapsed: -1))
        XCTAssertEqual(MeterSheen.cycleSeconds, 5.1, accuracy: 1e-9)
    }

    func testSheenFramesStopDuringTheRest() {
        XCTAssertEqual(MeterSheen.nextFrame(after: -0.4), 0)
        XCTAssertEqual(MeterSheen.nextFrame(after: 0, frameSeconds: 0.1), 0.1, accuracy: 1e-9)
        XCTAssertEqual(MeterSheen.nextFrame(after: 1.55, frameSeconds: 0.1), 1.6, accuracy: 1e-9)
        XCTAssertEqual(MeterSheen.nextFrame(after: 1.6, frameSeconds: 0.1), 5.1, accuracy: 1e-9)
        XCTAssertEqual(MeterSheen.nextFrame(after: 3, frameSeconds: 0.1), 5.1, accuracy: 1e-9)
        XCTAssertEqual(MeterSheen.nextFrame(after: 0.5, frameSeconds: 0), 5.1, accuracy: 1e-9)
    }

    func testSheenFramesAlwaysAdvance() {
        var elapsed = 0.0
        for _ in 0..<500 {
            let next = MeterSheen.nextFrame(after: elapsed)
            XCTAssertGreaterThan(next, elapsed)
            elapsed = next
        }
        XCTAssertGreaterThan(elapsed, MeterSheen.cycleSeconds * 3)
    }

    func testSheenBandCrossesTheWholeFill() {
        XCTAssertEqual(MeterSheen.bandOffset(progress: 0, fillWidth: 200, bandWidth: 36), -36)
        XCTAssertEqual(MeterSheen.bandOffset(progress: 1, fillWidth: 200, bandWidth: 36), 200)
        XCTAssertEqual(MeterSheen.bandOffset(progress: 2, fillWidth: 200, bandWidth: 36), 200)
        XCTAssertEqual(MeterSheen.bandOffset(progress: 0.5, fillWidth: 100, bandWidth: 20), 40)
    }
}
