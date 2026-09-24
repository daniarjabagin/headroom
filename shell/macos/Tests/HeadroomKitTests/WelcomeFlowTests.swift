import Foundation
import XCTest

@testable import HeadroomKit

@MainActor
private final class LoginItemSpy {
    var failures: [String?]
    private(set) var attempts = 0

    init(failures: [String?] = [nil]) {
        self.failures = failures
    }

    func enable() -> String? {
        attempts += 1
        return failures.isEmpty ? nil : failures.removeFirst()
    }
}

final class WelcomeFlowTests: XCTestCase {
    private var suiteName = ""
    private var defaults = UserDefaults.standard

    override func setUp() {
        super.setUp()
        suiteName = "headroom.tests.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName) ?? .standard
    }

    override func tearDown() {
        defaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    func testFlagStartsIncompleteAndPersists() {
        XCTAssertFalse(FirstRunFlag(defaults: defaults).isCompleted)
        FirstRunFlag(defaults: defaults).complete()
        XCTAssertTrue(FirstRunFlag(defaults: defaults).isCompleted)
        XCTAssertTrue(defaults.bool(forKey: "firstRunCompleted"))
    }

    @MainActor
    func testOpenAtLoginIsOnByDefaultAndRegistersOnContinue() {
        let spy = LoginItemSpy()
        let (flow, exits) = makeFlow(spy)
        XCTAssertTrue(flow.openAtLogin)
        flow.choose(.openHeadroom)
        XCTAssertEqual(spy.attempts, 1)
        XCTAssertEqual(exits(), [.openHeadroom])
        XCTAssertTrue(flow.isFinished)
        XCTAssertTrue(FirstRunFlag(defaults: defaults).isCompleted)
    }

    @MainActor
    func testTurningOpenAtLoginOffSkipsRegistration() {
        let spy = LoginItemSpy()
        let (flow, exits) = makeFlow(spy)
        flow.openAtLogin = false
        flow.choose(.settings)
        XCTAssertEqual(spy.attempts, 0)
        XCTAssertEqual(exits(), [.settings])
        XCTAssertTrue(FirstRunFlag(defaults: defaults).isCompleted)
    }

    @MainActor
    func testFailureStaysOpenUntilRetrySucceedsOrToggleIsOff() {
        let spy = LoginItemSpy(failures: ["Operation not permitted", "Operation not permitted"])
        let (flow, exits) = makeFlow(spy)
        flow.choose(.openHeadroom)
        XCTAssertEqual(flow.failure, "Operation not permitted")
        XCTAssertFalse(flow.isFinished)
        XCTAssertFalse(FirstRunFlag(defaults: defaults).isCompleted)
        flow.choose(.openHeadroom)
        XCTAssertEqual(flow.failure, "Operation not permitted")
        flow.openAtLogin = false
        XCTAssertNil(flow.failure)
        flow.choose(.openHeadroom)
        XCTAssertEqual(spy.attempts, 2)
        XCTAssertEqual(exits(), [.openHeadroom])
    }

    @MainActor
    func testRetryAfterFailureCanSucceed() {
        let spy = LoginItemSpy(failures: ["Busy", nil])
        let (flow, exits) = makeFlow(spy)
        flow.choose(.settings)
        flow.choose(.settings)
        XCTAssertNil(flow.failure)
        XCTAssertEqual(exits(), [.settings])
    }

    @MainActor
    func testDismissCompletesWithoutRegisteringAndFinishesOnce() {
        let spy = LoginItemSpy()
        let (flow, exits) = makeFlow(spy)
        flow.choose(.dismissed)
        flow.choose(.openHeadroom)
        XCTAssertEqual(spy.attempts, 0)
        XCTAssertEqual(exits(), [.dismissed])
        XCTAssertTrue(FirstRunFlag(defaults: defaults).isCompleted)
    }

    @MainActor
    private func makeFlow(_ spy: LoginItemSpy) -> (WelcomeFlow, () -> [WelcomeExit]) {
        let flow = WelcomeFlow(flag: FirstRunFlag(defaults: defaults)) { spy.enable() }
        var exits: [WelcomeExit] = []
        flow.onFinish = { exits.append($0) }
        return (flow, { exits })
    }
}
