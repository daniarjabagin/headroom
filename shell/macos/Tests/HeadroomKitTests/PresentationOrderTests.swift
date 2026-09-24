import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationOrderTests: XCTestCase {
    func testMoveItem() {
        XCTAssertEqual(AccountOrder.moveItem(["a", "b", "c"], from: 0, to: 2), ["b", "c", "a"])
        XCTAssertEqual(AccountOrder.moveItem(["a", "b", "c"], from: 2, to: 0), ["c", "a", "b"])
        XCTAssertEqual(AccountOrder.moveItem(["a", "b"], from: 1, to: 1), ["a", "b"])
        XCTAssertEqual(AccountOrder.moveItem(["a", "b"], from: 5, to: 0), ["a", "b"])
        XCTAssertEqual(AccountOrder.moveItem(["a", "b"], from: 0, to: 9), ["b", "a"])
    }

    func testMergeOrderKeepsHiddenAccountsInPlace() {
        XCTAssertEqual(
            AccountOrder.mergeOrder(all: ["a", "hidden", "b", "c"], visible: ["c", "a", "b"]),
            ["c", "hidden", "a", "b"])
    }

    func testDropIndexAndIndicator() {
        let others = [SectionSpan(top: 0, bottom: 100), SectionSpan(top: 114, bottom: 200)]
        XCTAssertEqual(AccountOrder.dropIndex(others: others, pointer: 20), 0)
        XCTAssertEqual(AccountOrder.dropIndex(others: others, pointer: 60), 1)
        XCTAssertEqual(AccountOrder.dropIndex(others: others, pointer: 500), 2)
        XCTAssertEqual(AccountOrder.indicatorPosition(others: others, target: 0, from: 2, gap: 14), -7)
        XCTAssertEqual(AccountOrder.indicatorPosition(others: others, target: 1, from: 0, gap: 14), 107)
        XCTAssertEqual(AccountOrder.indicatorPosition(others: others, target: 2, from: 0, gap: 14), 207)
        XCTAssertNil(AccountOrder.indicatorPosition(others: others, target: 1, from: 1, gap: 14))
        XCTAssertNil(AccountOrder.indicatorPosition(others: [], target: 0, from: 1, gap: 14))
    }
}
