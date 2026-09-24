import Foundation
import XCTest

@testable import HeadroomKit

enum Build {
    static let utc = TimeZone(identifier: "UTC") ?? .current
    static let english = DisplayFormatter(language: .en, timeZone: utc)
    static let russian = DisplayFormatter(language: .ru, timeZone: utc)

    static func now() throws -> Timestamp {
        try Fixture.timestamp("2026-09-23T10:00:00Z")
    }

    static func display(
        valueMode: String = "left", resetFormat: String = "countdown", showForecast: Bool = true,
        showTrend: Bool = true, showAccountSpend: Bool = true, showSpend: Bool = true
    ) throws -> DisplaySettings {
        try Fixture.decode(
            DisplaySettings.self,
            json: """
                {"theme":"system","language":"system","value_mode":"\(valueMode)","reset_format":"\(resetFormat)",
                "panel_label":"percent","show_spend":\(showSpend),"show_account_spend":\(showAccountSpend),
                "show_trend":\(showTrend),"show_forecast":\(showForecast),"translucent":false,"hidden_windows":{}}
                """)
    }

    static func window(
        id: String = "session", used: Double = 40, tone: String = "good", severity: String = "healthy",
        even: String = "null", projected: String = "null", spare: String = "null", runsOut: String = "null",
        resetsAt: String = "\"2026-09-23T12:00:00Z\"", hidden: Bool = false
    ) throws -> QuotaWindow {
        try Fixture.decode(
            QuotaWindow.self,
            json: windowJSON(
                id: id, used: used, tone: tone, severity: severity, even: even, projected: projected, spare: spare,
                runsOut: runsOut, resetsAt: resetsAt, hidden: hidden))
    }

    static func windowJSON(
        id: String = "session", used: Double = 40, tone: String = "good", severity: String = "healthy",
        even: String = "null", projected: String = "null", spare: String = "null", runsOut: String = "null",
        resetsAt: String = "\"2026-09-23T12:00:00Z\"", hidden: Bool = false
    ) -> String {
        """
        {"id":"\(id)","label":"\(id.capitalized)","used_percent":\(used),"remaining_percent":\(max(0, 100 - used)),
        "resets_at":\(resetsAt),"period_seconds":18000,"tone":"\(tone)","hidden":\(hidden),
        "pace":{"severity":"\(severity)","even_pace_percent":\(even),"projected_percent":\(projected),
        "spare_percent":\(spare),"runs_out_at":\(runsOut)}}
        """
    }

    static func accountJSON(
        id: String, provider: String = "codex", label: String = "null", email: String = "null",
        status: String = "fresh", error: String = "null", plan: String = "\"Pro\"", updatedAt: String = "null",
        windows: [String] = [], balances: String = "[]", notices: String = "[]", hidden: Bool = false,
        usageHome: String = "~/.codex"
    ) -> String {
        """
        {"id":"\(id)","provider":"\(provider)","provider_name":"\(provider.capitalized)","label":\(label),
        "email":\(email),"plan":\(plan),"hidden":\(hidden),"owner":"cli","status":"\(status)","error":\(error),
        "updated_at":\(updatedAt),"source":null,"windows":[\(windows.joined(separator: ","))],
        "balances":\(balances),"notices":\(notices),"usage_home":"\(usageHome)"}
        """
    }

    static func state(accounts: [String], offline: Bool = false, display: String? = nil) throws -> DaemonState {
        var base = try JSONSerialization.jsonObject(with: Fixture.data("state_empty")) as? [String: Any] ?? [:]
        base["accounts"] = try accounts.map { try JSONSerialization.jsonObject(with: Data($0.utf8)) }
        base["offline"] = offline
        if let display { base["display"] = try JSONSerialization.jsonObject(with: Data(display.utf8)) }
        let data = try JSONSerialization.data(withJSONObject: base)
        return try JSONDecoder().decode(DaemonState.self, from: data)
    }

    static func full() throws -> DaemonState {
        try Fixture.decode(DaemonState.self, "state_full")
    }
}

extension Build {
    static func mutated(_ fixture: String, _ edit: (inout [String: Any]) -> Void) throws -> DaemonState {
        var object = try XCTUnwrap(JSONSerialization.jsonObject(with: Fixture.data(fixture)) as? [String: Any])
        edit(&object)
        return try JSONDecoder().decode(DaemonState.self, from: JSONSerialization.data(withJSONObject: object))
    }

    static func combined(_ edit: (inout [String: Any]) -> Void) throws -> DaemonState {
        try mutated("state_combined", edit)
    }

    static func combined(headline: [String: Any], label: String = "percent") throws -> DaemonState {
        try combined { object in
            object["headline"] = headline
            var display = object["display"] as? [String: Any] ?? [:]
            display["panel_label"] = label
            object["display"] = display
        }
    }

    static func combinedHeadline(window: String, count: Int) -> [String: Any] {
        [
            "account_id": "codex:work", "provider": "codex", "provider_name": "Codex", "account_label": NSNull(),
            "window": window, "window_label": window.capitalized, "used_percent": 37.5, "remaining_percent": 62.5,
            "tone": "good", "combined": true, "account_count": count,
        ]
    }

    static func setSessionPace(_ object: inout [String: Any], _ pace: [String: Any]) {
        guard var groups = object["combined"] as? [[String: Any]], var group = groups.first,
            var windows = group["windows"] as? [[String: Any]], var session = windows.first
        else { return }
        let empty: [String: Any] = [
            "even_pace_percent": NSNull(), "projected_percent": NSNull(), "spare_percent": NSNull(),
            "runs_out_at": NSNull(),
        ]
        session["pace"] = empty.merging(pace) { _, new in new }
        windows[0] = session
        group["windows"] = windows
        groups[0] = group
        object["combined"] = groups
    }
}
