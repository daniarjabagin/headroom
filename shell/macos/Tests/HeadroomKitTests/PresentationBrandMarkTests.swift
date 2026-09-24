import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationBrandMarkTests: XCTestCase {
    private var repositoryRoot: URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent()
    }

    func testEverySymbolicCopyInTheRepositoryMatchesTheMark() throws {
        let copies = [
            "assets/brand/headroom-symbolic.svg",
            "shell/gnome/icons/headroom-symbolic.svg",
            "shell/plasma/package/contents/icons/headroom-symbolic.svg",
        ]
        for copy in copies {
            let text = try String(contentsOf: repositoryRoot.appendingPathComponent(copy), encoding: .utf8)
            XCTAssertEqual(try SVGIcon.parse(text), BrandMark.icon, copy)
        }
    }

    func testTheMarkIsFourClosedPlatesCentredInItsBox() {
        let icon = BrandMark.icon
        XCTAssertEqual(icon.commands.filter { $0 == .close }.count, 4)
        let points = icon.commands.compactMap { command -> PlanePoint? in
            switch command {
            case .move(let point), .line(let point): point
            case .cubic, .close: nil
            }
        }
        let xs = points.map(\.x)
        let ys = points.map(\.y)
        XCTAssertEqual((xs.min() ?? 0) + (xs.max() ?? 0), icon.width, accuracy: 1e-9)
        XCTAssertEqual((ys.min() ?? 0) + (ys.max() ?? 0), icon.height, accuracy: 1e-9)
    }
}
