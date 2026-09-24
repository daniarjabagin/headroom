import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationSVGTests: XCTestCase {
    private var iconDirectory: URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("Sources/HeadroomUI/Resources/ProviderIcons")
    }

    func testEveryBundledProviderIconParsesInsideItsViewBox() throws {
        let files = try FileManager.default.contentsOfDirectory(atPath: iconDirectory.path)
            .filter { $0.hasSuffix(".svg") }
        XCTAssertEqual(files.count, 12)
        for file in files {
            let text = try String(contentsOf: iconDirectory.appendingPathComponent(file), encoding: .utf8)
            let icon = try SVGIcon.parse(text)
            XCTAssertEqual(icon.width, 24, file)
            let points = icon.commands.flatMap(endpoints)
            XCTAssertGreaterThan(points.count, 3, file)
            for point in points {
                XCTAssertTrue((-0.01...24.01).contains(point.x) && (-0.01...24.01).contains(point.y), "\(file) \(point)")
            }
            XCTAssertNotNil(ProviderStyle.iconResource(for: String(file.dropLast(4))))
        }
    }

    func testRelativeCommandsAndImplicitRepeats() throws {
        let commands = try SVGPath.parse("m1 1 2 0h1v1H1.5.5L3-1z")
        XCTAssertEqual(
            commands,
            [
                .move(PlanePoint(x: 1, y: 1)), .line(PlanePoint(x: 3, y: 1)), .line(PlanePoint(x: 4, y: 1)),
                .line(PlanePoint(x: 4, y: 2)), .line(PlanePoint(x: 1.5, y: 2)), .line(PlanePoint(x: 0.5, y: 2)),
                .line(PlanePoint(x: 3, y: -1)), .close,
            ])
    }

    func testSmoothCurvesReflectTheLastControlPoint() throws {
        let commands = try SVGPath.parse("M0 0C1 0 2 1 2 2S3 4 4 4")
        XCTAssertEqual(
            commands.last, .cubic(PlanePoint(x: 2, y: 3), PlanePoint(x: 3, y: 4), PlanePoint(x: 4, y: 4)))
        let quadratic = try SVGPath.parse("M0 0Q3 0 3 3")
        XCTAssertEqual(
            quadratic.last, .cubic(PlanePoint(x: 2, y: 0), PlanePoint(x: 3, y: 1), PlanePoint(x: 3, y: 3)))
    }

    func testArcBecomesCubicsThatEndExactly() throws {
        let commands = try SVGPath.parse("M0 10a10 10 0 0 1 20 0")
        XCTAssertEqual(commands.count, 3)
        guard case .cubic(_, _, let middle) = commands[1], case .cubic(_, _, let end) = commands[2] else {
            return XCTFail("expected cubic segments")
        }
        XCTAssertEqual(middle.x, 10, accuracy: 1e-9)
        XCTAssertEqual(middle.y, 0, accuracy: 1e-9)
        XCTAssertEqual(end, PlanePoint(x: 20, y: 10))
        let compactFlags = try SVGPath.parse("M0 0a1 1 0 0120 0")
        XCTAssertEqual(compactFlags.count, 3)
    }

    func testMalformedDataFails() {
        XCTAssertThrowsError(try SVGPath.parse("L1 1"))
        XCTAssertThrowsError(try SVGPath.parse("M1"))
        XCTAssertThrowsError(try SVGPath.parse("M0 0 X1 1"))
        XCTAssertThrowsError(try SVGIcon.parse("<svg></svg>"))
        XCTAssertThrowsError(try SVGPath.parse("M0 0a1 1 0 2 0 3 3"))
    }

    private func endpoints(_ command: SVGPathCommand) -> [PlanePoint] {
        switch command {
        case .move(let point), .line(let point): [point]
        case .cubic(_, _, let point): [point]
        case .close: []
        }
    }
}
