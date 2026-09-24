import Foundation
import XCTest

@testable import HeadroomKit

final class ResourceLocatorTests: XCTestCase {
    private let root = FileManager.default.temporaryDirectory.appendingPathComponent("resources-\(UUID().uuidString)")

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: root)
    }

    private func makeDirectory(_ path: String) throws -> URL {
        let url = root.appendingPathComponent(path, isDirectory: true)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    private func locate(_ roots: [URL]) -> URL? {
        ResourceLocator.directory(named: "ProviderIcons", inBundle: ResourceLocator.uiBundleName, roots: roots)
    }

    func testFindsTheFlatSwiftPMBundleInsideTheAppResources() throws {
        let resources = try makeDirectory("Headroom.app/Contents/Resources")
        let icons = try makeDirectory("Headroom.app/Contents/Resources/HeadroomMac_HeadroomUI.bundle/ProviderIcons")
        let app = root.appendingPathComponent("Headroom.app")
        XCTAssertEqual(locate([resources, app])?.standardizedFileURL, icons.standardizedFileURL)
    }

    func testFindsAMacOSStyleBundleWithContentsResources() throws {
        let resources = try makeDirectory("Headroom.app/Contents/Resources")
        let icons = try makeDirectory(
            "Headroom.app/Contents/Resources/HeadroomMac_HeadroomUI.bundle/Contents/Resources/ProviderIcons")
        XCTAssertEqual(locate([resources])?.standardizedFileURL, icons.standardizedFileURL)
    }

    func testFallsBackToTheBundleNextToTheExecutable() throws {
        let build = try makeDirectory("release")
        let icons = try makeDirectory("release/HeadroomMac_HeadroomUI.bundle/ProviderIcons")
        let missing = root.appendingPathComponent("Headroom.app/Contents/Resources")
        XCTAssertEqual(locate([missing, build])?.standardizedFileURL, icons.standardizedFileURL)
    }

    func testMissingBundleYieldsNil() throws {
        let empty = try makeDirectory("empty")
        XCTAssertNil(locate([empty]))
    }
}
