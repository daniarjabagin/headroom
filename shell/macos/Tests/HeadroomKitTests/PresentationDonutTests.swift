import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationDonutTests: XCTestCase {
    private let geometry = DonutGeometry(size: 104)

    func testGeometryFollowsTheDesignRatios() {
        XCTAssertEqual(geometry.outer, 52)
        XCTAssertEqual(geometry.inner, 52 * 0.618, accuracy: 1e-9)
        XCTAssertEqual(geometry.gap, 2, accuracy: 1e-9)
        XCTAssertEqual(geometry.corner, 0.15 * 52 * (1 - 0.618), accuracy: 1e-9)
    }

    func testTinySharesAreRaisedToTheMinimumAndTheRestRescaled() {
        let fractions = DonutGeometry.visibleFractions([1_000_000, 1])
        XCTAssertEqual(fractions.reduce(0, +), 1, accuracy: 1e-9)
        XCTAssertEqual(fractions[1], DonutGeometry.minimumFraction, accuracy: 1e-12)
        XCTAssertEqual(DonutGeometry.visibleFractions([5]), [1])
        XCTAssertEqual(DonutGeometry.visibleFractions([0, 0]), [0, 0])
        XCTAssertEqual(DonutGeometry.visibleFractions([]), [])
        let even = DonutGeometry.visibleFractions([1, 1, 1])
        for fraction in even { XCTAssertEqual(fraction, 1.0 / 3, accuracy: 1e-12) }
    }

    func testSegmentsStartAtTwelveAndRespectReveal() {
        let segments = DonutGeometry.segments([0.25, 0.75])
        XCTAssertEqual(segments.count, 2)
        XCTAssertEqual(segments[0].start, -Double.pi / 2, accuracy: 1e-12)
        XCTAssertEqual(segments[0].end, 0, accuracy: 1e-12)
        XCTAssertTrue(segments.allSatisfy(\.gap))
        let half = DonutGeometry.segments([0.25, 0.75], reveal: 0.5)
        XCTAssertEqual(half.last?.end ?? 0, Double.pi / 2, accuracy: 1e-12)
        XCTAssertTrue(DonutGeometry.segments([0.5, 0.5], reveal: 0).isEmpty)
        let single = DonutGeometry.segments([1])
        XCTAssertEqual(single.map(\.gap), [false])
        XCTAssertEqual(single[0].end - single[0].start, 2 * Double.pi, accuracy: 1e-12)
    }

    func testSingleProviderDrawsAFullRingWithAHole() throws {
        let outline = try XCTUnwrap(geometry.outline(DonutGeometry.segments([1])[0]))
        XCTAssertEqual(outline.count, 2)
        for point in outline[0] { XCTAssertEqual(radius(point), geometry.outer, accuracy: 1e-9) }
        for point in outline[1] { XCTAssertEqual(radius(point), geometry.inner, accuracy: 1e-9) }
    }

    func testSectorsStayInsideTheRingAndKeepAnEvenGap() throws {
        let segments = DonutGeometry.segments([0.5, 0.5])
        let right = try XCTUnwrap(geometry.outline(segments[0]))
        let left = try XCTUnwrap(geometry.outline(segments[1]))
        XCTAssertEqual(right.count, 1)
        for point in right[0] + left[0] {
            XCTAssertLessThanOrEqual(radius(point), geometry.outer + 1e-9)
            XCTAssertGreaterThanOrEqual(radius(point), geometry.inner - 1e-9)
        }
        let halfGap = geometry.gap / 2
        XCTAssertTrue(right[0].allSatisfy { $0.x >= geometry.center + halfGap - 1e-9 })
        XCTAssertTrue(left[0].allSatisfy { $0.x <= geometry.center - halfGap + 1e-9 })
        XCTAssertEqual(right[0].map(\.x).min() ?? 0, geometry.center + halfGap, accuracy: 1e-6)
    }

    func testSliverWithoutRoomIsSkipped() {
        let sliver = DonutSegment(index: 0, start: 0, end: 0.001, gap: true)
        XCTAssertNil(geometry.outline(sliver))
    }

    private func radius(_ point: PlanePoint) -> Double {
        hypot(point.x - geometry.center, point.y - geometry.center)
    }
}
