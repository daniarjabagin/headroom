import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationMotionTests: XCTestCase {
    private let start = Date(timeIntervalSince1970: 1_000)

    private func at(_ seconds: TimeInterval) -> Date {
        start.addingTimeInterval(seconds)
    }

    func testRefreshTrackerHoldsTheSpinAndShowsFailures() {
        var tracker = RefreshTracker()
        XCTAssertTrue(tracker.press(at: at(0)))
        XCTAssertFalse(tracker.press(at: at(0.1)))
        XCTAssertEqual(tracker.mode(at: at(0.1)), .busy)
        tracker.settle(succeeded: true, at: at(0.2))
        XCTAssertEqual(tracker.mode(at: at(0.5)), .busy)
        XCTAssertEqual(tracker.nextChange(after: at(0.5)), at(0.8))
        XCTAssertEqual(tracker.mode(at: at(0.9)), .idle)
        XCTAssertTrue(tracker.press(at: at(1)))
        tracker.settle(succeeded: false, at: at(1.1))
        XCTAssertEqual(tracker.mode(at: at(1.2)), .failed)
        XCTAssertEqual(tracker.mode(at: at(2.2)), .idle)
        tracker.setDaemonBusy(true)
        XCTAssertEqual(tracker.mode(at: at(3)), .busy)
        XCTAssertNil(tracker.nextChange(after: at(3)))
    }

    func testSpinEasesInCruisesAndSettlesAtRest() {
        var spin = SpinMotion()
        XCTAssertEqual(spin.angle(at: at(0)), 0)
        spin.start(at: at(0))
        XCTAssertEqual(spin.angle(at: at(0.35)), 22.5, accuracy: 1e-3)
        XCTAssertEqual(spin.angle(at: at(0.7)), 90, accuracy: 1e-3)
        XCTAssertEqual(spin.angle(at: at(2.1)), 450, accuracy: 1e-3)
        spin.stop(at: at(2.1))
        XCTAssertFalse(spin.isSpinning)
        let rest = try? XCTUnwrap(spin.restsAt())
        XCTAssertEqual(rest?.timeIntervalSince(start) ?? 0, 2.1 + 0.7 + 0.7, accuracy: 1e-9)
        XCTAssertTrue(spin.isAnimating(at: at(3)))
        XCTAssertEqual(SpinMotion.resting(spin.angle(at: at(10))), 0, accuracy: 1e-9)
        XCTAssertFalse(spin.isAnimating(at: at(10)))
    }

    func testStopNearRestTakesAnExtraTurnInsteadOfSnapping() {
        var spin = SpinMotion()
        spin.start(at: at(0))
        let almost = 0.7 + 210.0 / 360 * 1.4
        spin.stop(at: at(almost))
        XCTAssertEqual(spin.angle(at: at(almost + 5)), 720, accuracy: 1e-6)
    }

    func testStopAtRestIsImmediate() {
        var spin = SpinMotion()
        spin.start(at: at(0))
        spin.stop(at: at(0))
        XCTAssertFalse(spin.isAnimating(at: at(0)))
        XCTAssertEqual(spin.angle(at: at(1)), 0)
    }

    func testShakeOscillatesAndEndsCentred() {
        XCTAssertEqual(ShakeMotion.offset(elapsed: 0), 0)
        XCTAssertEqual(ShakeMotion.offset(elapsed: 0.066), 3, accuracy: 1e-9)
        XCTAssertEqual(ShakeMotion.offset(elapsed: 0.132), -3, accuracy: 1e-9)
        XCTAssertEqual(ShakeMotion.offset(elapsed: 0.099), 0, accuracy: 1e-9)
        XCTAssertEqual(ShakeMotion.offset(elapsed: ShakeMotion.duration + 1), 0)
    }
}
