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
    func testStartCompletesOnboardingAndMovesToTheMenuBarStep() {
        let spy = LoginItemSpy()
        var completed = 0
        let (flow, exits) = makeFlow(spy, steps: [.found, .menuBar]) { completed += 1 }
        XCTAssertEqual(flow.step, .found)
        flow.choose(.openHeadroom)
        XCTAssertEqual(flow.step, .found)
        flow.start()
        XCTAssertEqual(completed, 1)
        XCTAssertEqual(flow.step, .menuBar)
        XCTAssertFalse(FirstRunFlag(defaults: defaults).isCompleted)
        flow.start()
        XCTAssertEqual(completed, 1)
        flow.choose(.openHeadroom)
        XCTAssertEqual(exits(), [.openHeadroom])
        XCTAssertTrue(FirstRunFlag(defaults: defaults).isCompleted)
    }

    @MainActor
    func testStartWithoutAMenuBarStepOpensHeadroom() {
        var completed = 0
        let (flow, exits) = makeFlow(LoginItemSpy(), steps: [.found]) { completed += 1 }
        flow.start()
        XCTAssertEqual(completed, 1)
        XCTAssertEqual(exits(), [.openHeadroom])
        XCTAssertFalse(FirstRunFlag(defaults: defaults).isCompleted)
    }

    @MainActor
    func testChooseLaterAndClosingLeaveOnboardingAndFirstRunOpen() {
        var completed = 0
        let (later, laterExits) = makeFlow(LoginItemSpy(), steps: [.found, .menuBar]) { completed += 1 }
        later.chooseLater()
        let (closed, closedExits) = makeFlow(LoginItemSpy(), steps: [.found, .menuBar]) { completed += 1 }
        closed.choose(.dismissed)
        XCTAssertEqual(completed, 0)
        XCTAssertEqual(laterExits(), [.dismissed])
        XCTAssertEqual(closedExits(), [.dismissed])
        XCTAssertFalse(FirstRunFlag(defaults: defaults).isCompleted)
    }

    func testPlanWaitsForSettingsAndOffersOnboardingOnlyToRelease06() throws {
        let pending = try Fixture.decode(
            Settings.self,
            json: SettingsTests.release06.replacingOccurrences(of: #""completed": true"#, with: #""completed": false"#))
        let done = try Fixture.decode(Settings.self, json: SettingsTests.release06)
        let legacy = try Fixture.decode(Settings.self, json: SettingsTests.legacy)
        XCTAssertEqual(WelcomePlan.decide(firstRunCompleted: false, settings: nil, timedOut: false), .wait)
        XCTAssertEqual(WelcomePlan.decide(firstRunCompleted: false, settings: nil, timedOut: true), .show([.menuBar]))
        XCTAssertEqual(WelcomePlan.decide(firstRunCompleted: true, settings: nil, timedOut: true), .wait)
        XCTAssertEqual(
            WelcomePlan.decide(firstRunCompleted: false, settings: pending, timedOut: false), .show([.found, .menuBar]))
        XCTAssertEqual(WelcomePlan.decide(firstRunCompleted: true, settings: pending, timedOut: false), .show([.found]))
        XCTAssertEqual(WelcomePlan.decide(firstRunCompleted: true, settings: done, timedOut: false), .skip)
        XCTAssertEqual(WelcomePlan.decide(firstRunCompleted: true, settings: legacy, timedOut: false), .skip)
        XCTAssertEqual(
            WelcomePlan.decide(firstRunCompleted: false, settings: legacy, timedOut: false), .show([.menuBar]))
    }

    @MainActor
    private func makeFlow(
        _ spy: LoginItemSpy, steps: [WelcomeStep] = [.menuBar], completeOnboarding: @escaping @MainActor () -> Void = {}
    ) -> (WelcomeFlow, () -> [WelcomeExit]) {
        let flow = WelcomeFlow(
            flag: FirstRunFlag(defaults: defaults), steps: steps, completeOnboarding: completeOnboarding
        ) { spy.enable() }
        var exits: [WelcomeExit] = []
        flow.onFinish = { exits.append($0) }
        return (flow, { exits })
    }
}
