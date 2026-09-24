import Foundation
import XCTest

@testable import HeadroomKit

@MainActor
private final class FakeUpdateControls: UpdateControls {
    var status: UpdaterStatus
    private(set) var checks = 0

    init(status: UpdaterStatus) {
        self.status = status
    }

    func setAutomaticallyChecks(_ enabled: Bool) {
        status.automaticallyChecks = enabled
    }

    func checkNow() {
        checks += 1
        status.canCheck = false
    }
}

final class UpdatesModelTests: XCTestCase {
    private let lastCheck = Date(timeIntervalSince1970: 1_790_000_000)

    @MainActor
    func testStartsUnavailableUntilConnected() {
        let model = UpdatesModel()
        XCTAssertEqual(model.status, .unavailable)
        model.checkNow()
        let controls = FakeUpdateControls(
            status: UpdaterStatus(automaticallyChecks: true, canCheck: true, lastCheck: lastCheck))
        model.connect(controls)
        XCTAssertEqual(model.status, controls.status)
        XCTAssertEqual(controls.checks, 0)
    }

    @MainActor
    func testToggleIsForwardedAndReflected() {
        let controls = FakeUpdateControls(
            status: UpdaterStatus(automaticallyChecks: true, canCheck: true, lastCheck: nil))
        let model = UpdatesModel()
        model.connect(controls)
        model.setAutomaticallyChecks(false)
        XCTAssertFalse(controls.status.automaticallyChecks)
        XCTAssertFalse(model.status.automaticallyChecks)
    }

    @MainActor
    func testCheckRunsOnlyWhenAllowed() {
        let controls = FakeUpdateControls(
            status: UpdaterStatus(automaticallyChecks: true, canCheck: true, lastCheck: nil))
        let model = UpdatesModel()
        model.connect(controls)
        model.checkNow()
        model.checkNow()
        XCTAssertEqual(controls.checks, 1)
        XCTAssertFalse(model.status.canCheck)
        controls.status = UpdaterStatus(automaticallyChecks: true, canCheck: true, lastCheck: lastCheck)
        model.refresh()
        XCTAssertEqual(model.status.lastCheck, lastCheck)
        XCTAssertTrue(model.status.canCheck)
    }

    func testLastCheckText() {
        let utc = TimeZone(identifier: "UTC") ?? .current
        let english = DisplayFormatter(language: .en, timeZone: utc)
        let russian = DisplayFormatter(language: .ru, timeZone: utc)
        let now = lastCheck.addingTimeInterval(3 * 3600 + 5 * 60)
        XCTAssertEqual(english.lastUpdateCheckText(nil, now: now), "Not checked yet")
        XCTAssertEqual(russian.lastUpdateCheckText(nil, now: now), "Ещё не проверялось")
        XCTAssertEqual(english.lastUpdateCheckText(now, now: now), "Last checked just now")
        XCTAssertEqual(english.lastUpdateCheckText(lastCheck, now: now), "Last checked 3h 5m ago")
        XCTAssertEqual(russian.lastUpdateCheckText(lastCheck, now: now), "Последняя проверка: 3 ч 5 мин назад")
    }
}
