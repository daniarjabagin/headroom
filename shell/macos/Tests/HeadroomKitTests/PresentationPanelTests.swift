import Foundation
import XCTest

@testable import HeadroomKit

#if canImport(CoreGraphics)
    import CoreGraphics
#endif

final class PresentationPanelTests: XCTestCase {
    private let screen = CGRect(x: 0, y: 0, width: 1440, height: 875)
    private let button = CGRect(x: 1000, y: 877, width: 40, height: 22)

    private func place(_ content: CGSize, below anchor: CGRect? = nil) -> CGRect {
        PanelPlacement.frame(content: content, below: anchor ?? button, within: screen, gap: 4, margin: 8)
    }

    func testCentersUnderTheButtonAndHangsFromItsBottomEdge() {
        let frame = place(CGSize(width: 320, height: 400))
        XCTAssertEqual(frame, CGRect(x: 860, y: 473, width: 320, height: 400))
    }

    func testTopEdgeStaysPutWhenTheContentHeightChanges() {
        let short = place(CGSize(width: 320, height: 200))
        let tall = place(CGSize(width: 320, height: 560))
        XCTAssertEqual(short.maxY, tall.maxY)
        XCTAssertEqual(short.minX, tall.minX)
        XCTAssertEqual(tall.height, 560)
    }

    func testHeightIsCappedAtSixHundredPoints() {
        let frame = place(CGSize(width: 320, height: 5000))
        XCTAssertEqual(frame.height, 600)
        XCTAssertEqual(frame.maxY, 873)
    }

    func testHeightKeepsClearOfTheBottomOnShortScreens() {
        let short = CGRect(x: 0, y: 0, width: 1280, height: 500)
        let frame = PanelPlacement.frame(
            content: CGSize(width: 320, height: 5000), below: CGRect(x: 600, y: 502, width: 40, height: 22),
            within: short, gap: 4, margin: 8)
        XCTAssertEqual(frame.height, 460)
    }

    func testMaxHeightTakesTheSmallerOfCapAndScreen() {
        XCTAssertEqual(PanelPlacement.maxHeight(visibleHeight: 956), 600)
        XCTAssertEqual(PanelPlacement.maxHeight(visibleHeight: 600), 560)
        XCTAssertEqual(PanelPlacement.maxHeight(visibleHeight: 20), 0)
    }

    func testStaysInsideTheScreenNearItsEdges() {
        let right = place(CGSize(width: 320, height: 300), below: CGRect(x: 1420, y: 877, width: 20, height: 22))
        XCTAssertEqual(right.maxX, 1432)
        let left = place(CGSize(width: 320, height: 300), below: CGRect(x: 0, y: 877, width: 20, height: 22))
        XCTAssertEqual(left.minX, 8)
    }

    func testTopNeverRisesAboveTheVisibleArea() {
        let frame = place(CGSize(width: 320, height: 300), below: CGRect(x: 600, y: 2000, width: 20, height: 22))
        XCTAssertEqual(frame.maxY, 875)
    }
}
