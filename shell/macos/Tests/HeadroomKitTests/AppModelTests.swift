import Foundation
import XCTest

@testable import HeadroomKit

final class AppModelTests: XCTestCase {
    @MainActor
    func testStateDrivesMenuBarAndAccounts() async throws {
        let model = AppModel(preferredLanguages: ["en-US"])
        XCTAssertEqual(model.menuBarContent, .glyph)
        model.apply(.connected)
        model.apply(.state(try Fixture.decode(DaemonState.self, "state_full")))
        XCTAssertEqual(model.phase, .connected)
        XCTAssertEqual(model.menuBarContent, .reading(text: "8%", fraction: 0.08))
        XCTAssertEqual(model.visibleAccounts.map(\.id), ["codex:work", "claude:main"])
    }

    @MainActor
    func testDisconnectFallsBackToGlyphButKeepsLastState() async throws {
        let model = AppModel(preferredLanguages: ["en"])
        model.apply(.state(try Fixture.decode(DaemonState.self, "state_full")))
        model.apply(.disconnected(.transport("gone")))
        XCTAssertEqual(model.phase, .disconnected)
        XCTAssertEqual(model.lastError, .transport("gone"))
        XCTAssertEqual(model.menuBarContent, .glyph)
        XCTAssertNotNil(model.state)
    }

    @MainActor
    func testSchemaMismatchIsSticky() async {
        let model = AppModel(preferredLanguages: ["en"])
        model.apply(.failure(.unsupportedSchema(2)))
        model.apply(.disconnected(nil))
        model.apply(.connecting)
        model.apply(.connected)
        XCTAssertEqual(model.phase, .incompatible(.schemaMismatch))
    }

    @MainActor
    func testRetriesDoNotFlickerBetweenConnectingAndDisconnected() async {
        let model = AppModel(preferredLanguages: ["en"])
        model.apply(.connecting)
        XCTAssertEqual(model.phase, .connecting)
        model.apply(.disconnected(.transport("refused")))
        model.apply(.connecting)
        XCTAssertEqual(model.phase, .disconnected)
        model.apply(.connected)
        XCTAssertEqual(model.phase, .connected)
    }

    @MainActor
    func testHelperMismatch() async {
        let model = AppModel(preferredLanguages: ["en"])
        model.markHelperMismatch()
        XCTAssertEqual(model.phase, .incompatible(.helperMismatch))
    }

    @MainActor
    func testDifferentServiceVersionIsNotRendered() async throws {
        let model = AppModel(preferredLanguages: ["en"], appVersion: "0.3.0")
        let snapshot = try Fixture.decode(DaemonState.self, "state_full")
        XCTAssertEqual(snapshot.appVersion, "0.0.0-snapshot")
        model.apply(.connected)
        model.apply(.state(snapshot))
        XCTAssertEqual(model.phase, .incompatible(.differentService))
        XCTAssertNil(model.state)
        XCTAssertEqual(model.menuBarContent, .glyph)
        model.apply(.connected)
        XCTAssertEqual(model.phase, .incompatible(.differentService))
        let matching = try Fixture.text("state_full").replacingOccurrences(of: "0.0.0-snapshot", with: "0.3.0")
        model.apply(.state(try Fixture.decode(DaemonState.self, json: matching)))
        XCTAssertEqual(model.phase, .connected)
        XCTAssertNotNil(model.state)
    }

    @MainActor
    func testMissingVersionOnEitherSideIsAccepted() async throws {
        let unversioned = try Fixture.text("state_full").replacingOccurrences(
            of: #""app_version": "0.0.0-snapshot","#, with: "")
        let state = try Fixture.decode(DaemonState.self, json: unversioned)
        XCTAssertNil(state.appVersion)
        let model = AppModel(preferredLanguages: ["en"], appVersion: "0.3.0")
        model.apply(.state(state))
        XCTAssertEqual(model.phase, .connected)
        let unbundled = AppModel(preferredLanguages: ["en"])
        unbundled.apply(.state(try Fixture.decode(DaemonState.self, "state_full")))
        XCTAssertEqual(unbundled.phase, .connected)
    }

    @MainActor
    func testLanguageFollowsDisplaySettingThenSystem() async throws {
        let model = AppModel(preferredLanguages: ["ru-RU"])
        XCTAssertEqual(model.formatter.language, .ru)
        model.apply(.state(try Fixture.decode(DaemonState.self, "state_empty")))
        XCTAssertEqual(model.formatter.language, .ru)
    }

    @MainActor
    func testSupervisorEventsSetServiceIssue() async {
        let model = AppModel(preferredLanguages: ["en"])
        model.apply(SupervisorEvent.exited(status: 1, restartIn: .seconds(2)))
        XCTAssertNotNil(model.serviceIssue)
        model.apply(SupervisorEvent.started)
        XCTAssertNil(model.serviceIssue)
    }
}

final class MenuBarContentTests: XCTestCase {
    func testUsedModeAndWindowLabel() throws {
        let json = try Fixture.text("state_full")
            .replacingOccurrences(of: #""value_mode": "left""#, with: #""value_mode": "used""#)
            .replacingOccurrences(of: #""panel_label": "percent""#, with: #""panel_label": "window""#)
        let state = try Fixture.decode(DaemonState.self, json: json)
        XCTAssertEqual(state.display.valueMode, .used)
        let content = MenuBarContent.make(state: state, formatter: DisplayFormatter(language: .ru))
        XCTAssertEqual(content, .reading(text: "Сессия", fraction: 0.92))
    }

    func testNoHeadlineShowsGlyph() throws {
        let state = try Fixture.decode(DaemonState.self, "state_empty")
        XCTAssertEqual(MenuBarContent.make(state: state, formatter: DisplayFormatter(language: .en)), .glyph)
    }
}
