import Foundation
import XCTest

@testable import HeadroomKit

final class RetryTrackerTests: XCTestCase {
    private let start = Date(timeIntervalSince1970: 1_000)

    func testRepeatedTapsWhileTheRequestIsInFlightSendOnce() {
        var tracker = RetryTracker()
        XCTAssertTrue(tracker.press("a", at: start))
        XCTAssertFalse(tracker.press("a", at: start.addingTimeInterval(0.2)))
        XCTAssertTrue(tracker.press("b", at: start.addingTimeInterval(0.2)))
        XCTAssertTrue(tracker.isPending("a", at: start.addingTimeInterval(0.2)))
    }

    func testDaemonRefreshingTakesOverTheBusyState() {
        var tracker = RetryTracker()
        _ = tracker.press("a", at: start)
        tracker.observe(refreshing: ["a"])
        XCTAssertFalse(tracker.isPending("a", at: start))
        XCTAssertTrue(tracker.press("a", at: start.addingTimeInterval(1)))
    }

    func testTapDuringARefreshIsSentAndClearsOnReply() {
        var tracker = RetryTracker()
        tracker.observe(refreshing: ["a"])
        XCTAssertTrue(tracker.press("a", at: start))
        tracker.settle("a", succeeded: true)
        XCTAssertFalse(tracker.isPending("a", at: start))
    }

    func testSuccessfulReplyBeforeTheStateChangeKeepsTheTapBusy() {
        var tracker = RetryTracker()
        _ = tracker.press("a", at: start)
        tracker.settle("a", succeeded: true)
        XCTAssertTrue(tracker.isPending("a", at: start.addingTimeInterval(0.1)))
    }

    func testFailedRequestFreesTheButton() {
        var tracker = RetryTracker()
        _ = tracker.press("a", at: start)
        tracker.settle("a", succeeded: false)
        XCTAssertFalse(tracker.isPending("a", at: start))
        XCTAssertTrue(tracker.press("a", at: start.addingTimeInterval(0.1)))
    }

    func testPendingTapExpires() {
        var tracker = RetryTracker()
        _ = tracker.press("a", at: start)
        let expiry = start.addingTimeInterval(RetryTracker.timeout)
        XCTAssertEqual(tracker.nextExpiry(after: start), expiry)
        XCTAssertFalse(tracker.isPending("a", at: expiry))
        XCTAssertNil(tracker.nextExpiry(after: expiry))
        XCTAssertTrue(tracker.press("a", at: expiry))
    }
}
