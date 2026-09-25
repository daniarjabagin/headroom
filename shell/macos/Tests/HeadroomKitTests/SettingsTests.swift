import Foundation
import XCTest

@testable import HeadroomKit

final class SettingsTests: XCTestCase {
    static let legacy = #"""
        {
          "refresh_interval_secs": 300,
          "notifications": { "almost_out": true, "cutting_it_close": true, "will_run_out": true, "reset": false },
          "headline": { "mode": "auto" },
          "reduced_motion": false,
          "display": {
            "theme": "system", "language": "system", "value_mode": "left", "reset_format": "countdown",
            "panel_label": "percent", "show_spend": true, "show_account_spend": true, "show_trend": true,
            "show_forecast": true, "translucent": false, "hidden_windows": {}
          }
        }
        """#

    static let release06 = #"""
        {
          "refresh_interval_secs": 600,
          "adaptive_refresh": false,
          "notifications": {
            "almost_out": true, "cutting_it_close": true, "will_run_out": true, "reset": false,
            "threshold_percent": 20, "provider_thresholds": { "claude": 30, "copilot": 0 },
            "quiet_hours": { "enabled": true, "from": "23:30", "to": "07:05", "allow_critical": false }
          },
          "headline": { "mode": "pinned", "account_id": "claude:main", "window": "session" },
          "reduced_motion": false,
          "display": {
            "theme": "dark", "language": "ru", "value_mode": "used", "reset_format": "exact",
            "panel_label": "none", "show_spend": true, "show_account_spend": true, "show_trend": true,
            "show_forecast": true, "translucent": false, "combine_accounts": true, "hidden_windows": {},
            "density": "compact", "time_format": "12h", "panel_mode": "several", "panel_indicator": "bar",
            "panel_limits": [{ "account_id": "claude:main", "window": "weekly" }],
            "panel_position": { "box": "left", "index": 2 },
            "spend_period": "7d", "spend_unit": "cost_per_mtok", "spend_breakdown": "projects",
            "starred_accounts": ["codex:work"], "collapse_unstarred": true, "hide_on_screen_share": false
          },
          "updates": { "check": true },
          "status_pages": { "enabled": true },
          "shortcuts": { "open": "<Super>u" },
          "logging": { "level": "debug" },
          "onboarding": { "completed": true }
        }
        """#

    func testLegacySettingsDecodeWithDefaultsForEveryNewKey() throws {
        let settings = try Fixture.decode(Settings.self, json: Self.legacy)
        XCTAssertEqual(settings.refreshIntervalSecs, 300)
        XCTAssertEqual(settings.headline, .auto)
        XCTAssertEqual(settings.features, .legacy)
        XCTAssertTrue(settings.adaptiveRefresh)
        XCTAssertEqual(settings.notifications.thresholdPercent, 10)
        XCTAssertEqual(settings.notifications.providerThresholds, [:])
        XCTAssertEqual(settings.notifications.quietHours, .standard)
        XCTAssertEqual(settings.notifications.quietHours.from.text, "22:00")
        XCTAssertEqual(settings.notifications.quietHours.to.text, "08:00")
        XCTAssertFalse(settings.statusPages.enabled)
        XCTAssertEqual(settings.shortcuts.open, "")
        XCTAssertEqual(settings.logging.level, .info)
        XCTAssertFalse(settings.onboarding.completed)
        let display = settings.display
        XCTAssertEqual(display.density, .normal)
        XCTAssertEqual(display.timeFormat, .auto)
        XCTAssertEqual(display.panelMode, .headline)
        XCTAssertEqual(display.panelIndicator, .ring)
        XCTAssertEqual(display.panelLimits, [])
        XCTAssertEqual(display.panelPosition, PanelPosition(box: .right, index: 0))
        XCTAssertEqual(display.spendPeriod, .last30Days)
        XCTAssertEqual(display.spendUnit, .cost)
        XCTAssertEqual(display.spendBreakdown, .models)
        XCTAssertEqual(display.starredAccounts, [])
        XCTAssertFalse(display.collapseUnstarred)
        XCTAssertTrue(display.hideOnScreenShare)
    }

    func testRelease06SettingsDecodeEveryKey() throws {
        let settings = try Fixture.decode(Settings.self, json: Self.release06)
        XCTAssertEqual(settings.features, .current)
        XCTAssertFalse(settings.adaptiveRefresh)
        XCTAssertEqual(settings.headline, .pinned(accountID: "claude:main", window: "session"))
        let notifications = settings.notifications
        XCTAssertEqual(notifications.thresholdPercent, 20)
        XCTAssertEqual(notifications.providerThresholds, ["claude": 30, "copilot": 0])
        XCTAssertEqual(notifications.threshold(provider: "claude"), 30)
        XCTAssertEqual(notifications.threshold(provider: "copilot"), 0)
        XCTAssertEqual(notifications.threshold(provider: "codex"), 20)
        XCTAssertEqual(
            notifications.quietHours,
            QuietHours(enabled: true, from: try time("23:30"), to: try time("07:05"), allowCritical: false))
        XCTAssertTrue(settings.statusPages.enabled)
        XCTAssertEqual(settings.shortcuts.open, "<Super>u")
        XCTAssertEqual(settings.logging.level, .debug)
        XCTAssertTrue(settings.onboarding.completed)
        let display = settings.display
        XCTAssertEqual(display.panelLabel, PanelLabel.none)
        XCTAssertEqual(display.density, .compact)
        XCTAssertEqual(display.timeFormat, .twelveHour)
        XCTAssertEqual(display.panelMode, .several)
        XCTAssertEqual(display.panelIndicator, .bar)
        XCTAssertEqual(display.panelLimits, [PanelLimit(accountID: "claude:main", window: "weekly")])
        XCTAssertEqual(display.panelPosition, PanelPosition(box: .left, index: 2))
        XCTAssertEqual(display.spendPeriod, .last7Days)
        XCTAssertEqual(display.spendUnit, .costPerMTok)
        XCTAssertEqual(display.spendBreakdown, .projects)
        XCTAssertEqual(display.starredAccounts, ["codex:work"])
        XCTAssertTrue(display.collapseUnstarred)
        XCTAssertFalse(display.hideOnScreenShare)
    }

    func testUnknownEnumValuesFallBackAndPartialQuietHoursKeepDefaults() throws {
        let json = Self.release06
            .replacingOccurrences(of: #""time_format": "12h""#, with: #""time_format": "36h""#)
            .replacingOccurrences(of: #""level": "debug""#, with: #""level": "trace""#)
            .replacingOccurrences(
                of: #"{ "enabled": true, "from": "23:30", "to": "07:05", "allow_critical": false }"#,
                with: #"{ "enabled": true }"#)
        let settings = try Fixture.decode(Settings.self, json: json)
        XCTAssertEqual(settings.display.timeFormat, .auto)
        XCTAssertEqual(settings.logging.level, .info)
        XCTAssertEqual(
            settings.notifications.quietHours,
            QuietHours(enabled: true, from: try time("22:00"), to: try time("08:00"), allowCritical: true))
    }

    func testMalformedQuietHoursTimeIsAnError() {
        let json = Self.release06.replacingOccurrences(of: #""from": "23:30""#, with: #""from": "7:30""#)
        XCTAssertThrowsError(try Fixture.decode(Settings.self, json: json))
    }

    func testTimeOfDayParsesOnlyTwoDigitClockTimes() {
        XCTAssertEqual(TimeOfDay("00:00")?.text, "00:00")
        XCTAssertEqual(TimeOfDay("23:59")?.text, "23:59")
        XCTAssertEqual(TimeOfDay(hour: 7, minute: 5)?.text, "07:05")
        for invalid in ["24:00", "12:60", "7:30", "07:5", "0730", "07:30:00", "", "a1:00", "+1:00", "١٢:٠٠"] {
            XCTAssertNil(TimeOfDay(invalid), invalid)
        }
        XCTAssertNil(TimeOfDay(hour: -1, minute: 0))
        XCTAssertTrue(try time("08:00") < time("22:00"))
    }

    func testPatchKeepsIntegersIntegral() throws {
        let patch: [String: JSONValue] = [
            "refresh_interval_secs": .integer(600), "display": .object(["hidden_windows": .object(["a": .null])]),
        ]
        XCTAssertEqual(
            try RPCCodec.encodeString(patch),
            #"{"display":{"hidden_windows":{"a":null}},"refresh_interval_secs":600}"#)
    }

    private func time(_ text: String) throws -> TimeOfDay {
        try XCTUnwrap(TimeOfDay(text))
    }
}
