import Foundation
import XCTest

@testable import HeadroomKit

final class SettingsRulesTests: XCTestCase {
    private func settings() throws -> Settings {
        try Fixture.decode(Settings.self, json: SettingsTests.release06)
    }

    private func problem(_ change: SettingsChange) throws -> SettingsProblem? {
        SettingsRules.problem(in: change.applied(to: try settings()))
    }

    func testStoredDocumentsAreValid() throws {
        XCTAssertNil(SettingsRules.problem(in: try settings()))
        XCTAssertNil(SettingsRules.problem(in: try Fixture.decode(Settings.self, json: SettingsTests.legacy)))
    }

    func testRangesMirrorTheDaemon() throws {
        var base = try settings()
        base.refreshIntervalSecs = 59
        XCTAssertEqual(SettingsRules.problem(in: base), .refreshInterval(59))
        base.refreshIntervalSecs = 3600
        base.notifications.thresholdPercent = 51
        XCTAssertEqual(SettingsRules.problem(in: base), .thresholdPercent(51))
        base.notifications.thresholdPercent = 50
        base.notifications.providerThresholds = ["claude": 51]
        XCTAssertEqual(SettingsRules.problem(in: base), .providerThreshold(provider: "claude", value: 51))
        base.notifications.providerThresholds = [" ": 10]
        XCTAssertEqual(SettingsRules.problem(in: base), .blankProviderThreshold)
        base.notifications.providerThresholds = ["copilot": 0]
        XCTAssertNil(SettingsRules.problem(in: base))
    }

    func testBlankIdsAreRejected() throws {
        var base = try settings()
        base.headline = .pinned(accountID: "claude:main", window: " ")
        XCTAssertEqual(SettingsRules.problem(in: base), .emptyHeadlineTarget)
        base.headline = .auto
        base.display.hiddenWindows = ["codex:a": [""]]
        XCTAssertEqual(SettingsRules.problem(in: base), .blankHiddenWindow)
        base.display.hiddenWindows = [:]
        base.display.starredAccounts = ["\t"]
        XCTAssertEqual(SettingsRules.problem(in: base), .blankStarredAccount)
        base.display.starredAccounts = []
        base.display.panelLimits = [PanelLimit(accountID: "claude:a", window: "")]
        XCTAssertEqual(SettingsRules.problem(in: base), .blankPanelLimit)
    }

    func testMoreThanThreeDistinctPanelLimitsIsInvalid() throws {
        let limits = ["a", "b", "c", "d"].map { PanelLimit(accountID: "claude:\($0)", window: "session") }
        XCTAssertEqual(try problem(.panel(.limits(limits))), .tooManyPanelLimits(4))
        XCTAssertNil(try problem(.panel(.limits(Array(limits.prefix(3)) + [limits[0]]))))
    }

    func testQuietHoursMustNotBeEmptyWhileEnabled() throws {
        let same = try XCTUnwrap(TimeOfDay("07:05"))
        XCTAssertEqual(try problem(.alerts(.quietHours(.from(same)))), .emptyQuietHours)
        var base = try settings()
        base.notifications.quietHours.enabled = false
        base.notifications.quietHours.from = same
        XCTAssertNil(SettingsRules.problem(in: base))
    }

    func testShortcutsFollowTheAcceleratorSyntax() throws {
        for valid in ["", "<Super>u", "<Control><Alt>h", "F12", "<Shift>Page_Up", "a"] {
            XCTAssertNil(try problem(.shortcut(valid)), valid)
        }
        for invalid in ["<Super>", "<>u", "<Super u", "<Sup3r>u", "Ctrl+U", "<Super>ü", " "] {
            XCTAssertEqual(try problem(.shortcut(invalid)), .invalidShortcut(invalid), invalid)
        }
        XCTAssertEqual(try problem(.shortcut("<Control>" + String(repeating: "a", count: 56))), .shortcutTooLong)
        XCTAssertNil(try problem(.shortcut("<Control>" + String(repeating: "a", count: 55))))
    }

    func testFeaturesFollowPanelItemsOrTheReleaseVersion() throws {
        XCTAssertTrue(try Fixture.decode(DaemonState.self, "state_full").features.release06)
        XCTAssertFalse(try Fixture.decode(DaemonState.self, json: StateSamples.legacy).features.release06)
        let versions: [(String?, Bool)] = [
            ("0.6.0", true), ("0.6.1", true), ("0.10.0", true), ("1.0.0", true), ("0.6.0-rc.1", true),
            ("0.5.9", false), ("0.0.0-snapshot", false), (nil, false), ("0.6", false), ("x.y.z", false),
        ]
        for (version, expected) in versions {
            XCTAssertEqual(DaemonFeatures.isRelease06(appVersion: version), expected, version ?? "nil")
        }
    }
}
