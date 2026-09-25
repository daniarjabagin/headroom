import Foundation
import XCTest

@testable import HeadroomKit

final class SettingsRelease06ChangeTests: XCTestCase {
    private func settings() throws -> Settings {
        try Fixture.decode(Settings.self, json: SettingsTests.release06)
    }

    private func patchText(_ change: SettingsChange) throws -> String {
        try RPCCodec.encodeString(change.patch)
    }

    func testDisplayKeyPatches() throws {
        let cases: [(SettingsChange, String)] = [
            (.panelLabel(.none), #"{"display":{"panel_label":"none"}}"#),
            (.density(.compact), #"{"display":{"density":"compact"}}"#),
            (.timeFormat(.twentyFourHour), #"{"display":{"time_format":"24h"}}"#),
            (.panel(.mode(.icon)), #"{"display":{"panel_mode":"icon"}}"#),
            (.panel(.indicator(.none)), #"{"display":{"panel_indicator":"none"}}"#),
            (.spend(.period(.last7Days)), #"{"display":{"spend_period":"7d"}}"#),
            (.spend(.unit(.costPerMTok)), #"{"display":{"spend_unit":"cost_per_mtok"}}"#),
            (.spend(.breakdown(.projects)), #"{"display":{"spend_breakdown":"projects"}}"#),
            (.collapseUnstarred(true), #"{"display":{"collapse_unstarred":true}}"#),
            (.hideOnScreenShare(false), #"{"display":{"hide_on_screen_share":false}}"#),
        ]
        for (change, expected) in cases {
            XCTAssertEqual(try patchText(change), expected)
        }
    }

    func testArraysAndPositionAreReplacedWhole() throws {
        let limits = [
            PanelLimit(accountID: "claude:a", window: "session"), PanelLimit(accountID: "claude:a", window: "session"),
            PanelLimit(accountID: " ", window: "weekly"), PanelLimit(accountID: "codex:b", window: "weekly"),
        ]
        XCTAssertEqual(
            try patchText(.panel(.limits(limits))),
            #"{"display":{"panel_limits":[{"account_id":"claude:a","window":"session"},"#
                + #"{"account_id":"codex:b","window":"weekly"}]}}"#)
        XCTAssertEqual(try patchText(.panel(.limits([]))), #"{"display":{"panel_limits":[]}}"#)
        XCTAssertEqual(
            try patchText(.panel(.position(PanelPosition(box: .center, index: 3)))),
            #"{"display":{"panel_position":{"box":"center","index":3}}}"#)
        XCTAssertEqual(
            try patchText(.starredAccounts(["codex:a", "", "codex:a", "claude:b"])),
            #"{"display":{"starred_accounts":["codex:a","claude:b"]}}"#)
    }

    func testTopLevelAndSectionPatches() throws {
        let cases: [(SettingsChange, String)] = [
            (.adaptiveRefresh(false), #"{"adaptive_refresh":false}"#),
            (.statusPages(true), #"{"status_pages":{"enabled":true}}"#),
            (.shortcut("<Control><Alt>h"), #"{"shortcuts":{"open":"<Control><Alt>h"}}"#),
            (.shortcut(""), #"{"shortcuts":{"open":""}}"#),
            (.logLevel(.warn), #"{"logging":{"level":"warn"}}"#),
            (.onboardingCompleted(true), #"{"onboarding":{"completed":true}}"#),
        ]
        for (change, expected) in cases {
            XCTAssertEqual(try patchText(change), expected)
        }
    }

    func testThresholdPatchesMergePerProviderAndNeverSendNullForOff() throws {
        XCTAssertEqual(try patchText(.alerts(.threshold(20))), #"{"notifications":{"threshold_percent":20}}"#)
        XCTAssertEqual(try patchText(.alerts(.threshold(0))), #"{"notifications":{"threshold_percent":1}}"#)
        XCTAssertEqual(try patchText(.alerts(.threshold(80))), #"{"notifications":{"threshold_percent":50}}"#)
        XCTAssertEqual(
            try patchText(.alerts(.providerThreshold(provider: "claude", percent: 20))),
            #"{"notifications":{"provider_thresholds":{"claude":20}}}"#)
        XCTAssertEqual(
            try patchText(.alerts(.providerThreshold(provider: "copilot", percent: 0))),
            #"{"notifications":{"provider_thresholds":{"copilot":0}}}"#)
        XCTAssertEqual(
            try patchText(.alerts(.providerThreshold(provider: "claude", percent: nil))),
            #"{"notifications":{"provider_thresholds":{"claude":null}}}"#)
        XCTAssertEqual(
            try patchText(.alerts(.providerThreshold(provider: "claude", percent: 99))),
            #"{"notifications":{"provider_thresholds":{"claude":50}}}"#)
    }

    func testQuietHoursPatchesMergeFieldByField() throws {
        let from = try XCTUnwrap(TimeOfDay("21:15"))
        let to = try XCTUnwrap(TimeOfDay("06:00"))
        let cases: [(QuietHoursField, String)] = [
            (.enabled(true), #"{"notifications":{"quiet_hours":{"enabled":true}}}"#),
            (.from(from), #"{"notifications":{"quiet_hours":{"from":"21:15"}}}"#),
            (.to(to), #"{"notifications":{"quiet_hours":{"to":"06:00"}}}"#),
            (.allowCritical(false), #"{"notifications":{"quiet_hours":{"allow_critical":false}}}"#),
        ]
        for (field, expected) in cases {
            XCTAssertEqual(try patchText(.alerts(.quietHours(field))), expected)
        }
    }

    func testAppliedMatchesEveryPatch() throws {
        let base = try settings()
        XCTAssertEqual(SettingsChange.density(.normal).applied(to: base).display.density, .normal)
        XCTAssertEqual(SettingsChange.timeFormat(.auto).applied(to: base).display.timeFormat, .auto)
        XCTAssertEqual(SettingsChange.panel(.mode(.icon)).applied(to: base).display.panelMode, .icon)
        XCTAssertEqual(SettingsChange.panel(.indicator(.ring)).applied(to: base).display.panelIndicator, .ring)
        let limit = PanelLimit(accountID: "codex:b", window: "session")
        XCTAssertEqual(SettingsChange.panel(.limits([limit, limit])).applied(to: base).display.panelLimits, [limit])
        let position = PanelPosition(box: .center, index: 1)
        XCTAssertEqual(SettingsChange.panel(.position(position)).applied(to: base).display.panelPosition, position)
        XCTAssertEqual(SettingsChange.spend(.period(.today)).applied(to: base).display.spendPeriod, .today)
        XCTAssertEqual(SettingsChange.spend(.unit(.tokens)).applied(to: base).display.spendUnit, .tokens)
        XCTAssertEqual(SettingsChange.spend(.breakdown(.models)).applied(to: base).display.spendBreakdown, .models)
        XCTAssertEqual(SettingsChange.starredAccounts(["a", "a"]).applied(to: base).display.starredAccounts, ["a"])
        XCTAssertFalse(SettingsChange.collapseUnstarred(false).applied(to: base).display.collapseUnstarred)
        XCTAssertTrue(SettingsChange.hideOnScreenShare(true).applied(to: base).display.hideOnScreenShare)
        XCTAssertTrue(SettingsChange.adaptiveRefresh(true).applied(to: base).adaptiveRefresh)
        XCTAssertFalse(SettingsChange.statusPages(false).applied(to: base).statusPages.enabled)
        XCTAssertEqual(SettingsChange.shortcut("").applied(to: base).shortcuts.open, "")
        XCTAssertEqual(SettingsChange.logLevel(.error).applied(to: base).logging.level, .error)
        XCTAssertFalse(SettingsChange.onboardingCompleted(false).applied(to: base).onboarding.completed)
        XCTAssertEqual(base.features, SettingsChange.density(.normal).applied(to: base).features)
    }

    func testAppliedAlertChanges() throws {
        let base = try settings()
        XCTAssertEqual(SettingsChange.alerts(.threshold(5)).applied(to: base).notifications.thresholdPercent, 5)
        let removed = SettingsChange.alerts(.providerThreshold(provider: "claude", percent: nil)).applied(to: base)
        XCTAssertEqual(removed.notifications.providerThresholds, ["copilot": 0])
        let added = SettingsChange.alerts(.providerThreshold(provider: "codex", percent: 10)).applied(to: base)
        XCTAssertEqual(added.notifications.providerThresholds, ["claude": 30, "copilot": 0, "codex": 10])
        let from = try XCTUnwrap(TimeOfDay("20:00"))
        let quiet = SettingsChange.alerts(.quietHours(.from(from))).applied(to: base).notifications.quietHours
        XCTAssertEqual(quiet.from, from)
        XCTAssertEqual(quiet.to.text, "07:05")
        XCTAssertFalse(
            SettingsChange.alerts(.quietHours(.enabled(false))).applied(to: base).notifications.quietHours.enabled)
        XCTAssertTrue(
            SettingsChange.alerts(.quietHours(.allowCritical(true))).applied(to: base).notifications.quietHours
                .allowCritical)
    }

    func testOnlyRelease06KeysNeedARelease06Service() {
        XCTAssertFalse(SettingsChange.theme(.dark).requiresRelease06)
        XCTAssertFalse(SettingsChange.panelLabel(.window).requiresRelease06)
        XCTAssertTrue(SettingsChange.panelLabel(.none).requiresRelease06)
        XCTAssertTrue(SettingsChange.density(.compact).requiresRelease06)
        XCTAssertTrue(SettingsChange.alerts(.threshold(20)).requiresRelease06)
        XCTAssertTrue(DaemonFeatures.legacy.allows(.theme(.dark)))
        XCTAssertFalse(DaemonFeatures.legacy.allows(.panelLabel(.none)))
        XCTAssertFalse(DaemonFeatures.legacy.allows(.logLevel(.debug)))
        XCTAssertTrue(DaemonFeatures.current.allows(.logLevel(.debug)))
    }

    func testStarredAccountsToggle() throws {
        let display = try settings().display
        XCTAssertTrue(display.isStarred(accountID: "codex:work"))
        XCTAssertFalse(display.isStarred(accountID: "claude:main"))
        XCTAssertEqual(display.starredAccounts(after: "claude:main", starred: true), ["codex:work", "claude:main"])
        XCTAssertEqual(display.starredAccounts(after: "codex:work", starred: false), [])
        XCTAssertEqual(display.starredAccounts(after: "codex:work", starred: true), ["codex:work"])
    }
}
