#if canImport(AppKit) && DEBUG
    import Foundation
    import HeadroomKit

    enum PreviewFixtures {
        static func state(offline: Bool = false, singleProvider: Bool = false) -> DaemonState? {
            let json = """
                {"version":1,"generated_at":"2026-09-23T10:00:00Z","next_refresh_at":"2026-09-23T10:03:00Z",
                "last_success_at":"2026-09-23T09:58:00Z","offline":\(offline),"display":\(display),"headline":null,
                "accounts":[\(accounts(offline: offline).joined(separator: ","))],
                "usage":[\(usage("codex", "Codex", "~/.codex", seed: 3)),\(usage("claude", "Claude", "~/.claude", seed: 7))],
                "spend":\(spend(singleProvider: singleProvider))}
                """
            return try? JSONDecoder().decode(DaemonState.self, from: Data(json.utf8))
        }

        static let empty: DaemonState? = {
            let json = """
                {"version":1,"generated_at":"2026-09-23T10:00:00Z","next_refresh_at":null,"last_success_at":null,
                "offline":false,"display":\(display),"headline":null,"accounts":[],"usage":[],
                "spend":{"today":\(period([])),"yesterday":\(period([])),"last_30_days":\(period([]))}}
                """
            return try? JSONDecoder().decode(DaemonState.self, from: Data(json.utf8))
        }()

        private static let display = """
            {"theme":"system","language":"en","value_mode":"left","reset_format":"countdown","panel_label":"percent",
            "show_spend":true,"show_account_spend":true,"show_trend":true,"show_forecast":true,"translucent":false,
            "hidden_windows":{}}
            """

        private static func accounts(offline: Bool) -> [String] {
            let network = #"{"kind":"network","message":"HTTP 503 from chatgpt.com"}"#
            return [
                account(
                    "codex:work", "codex", "Codex", label: "work", status: offline ? "error" : "fresh",
                    error: offline ? network : "null",
                    windows: [
                        window("session", 55, "warning", "close", even: 60, spare: 8.3, resets: "2026-09-23T12:00:00Z"),
                        window("weekly", 19, "good", "healthy", even: 30, spare: 47.5, resets: "2026-09-26T10:00:00Z"),
                    ],
                    balances: #"[{"id":"credits","label":"Credits","kind":"usd","usd_micros":12500000}]"#,
                    notices: #"[{"tone":"neutral","text":"Weekly limit shared with Codex Cloud"}]"#),
                account(
                    "claude:max", "claude", "Claude", label: "max", status: "refreshing", error: "null",
                    windows: [
                        window(
                            "session", 92, "critical", "running_out", even: 70, spare: nil,
                            resets: "2026-09-23T10:40:00Z",
                            runsOut: "2026-09-23T10:23:00Z")
                    ]),
                account(
                    "claude:personal", "claude", "Claude", label: "personal", status: "signed_out",
                    error: #"{"kind":"sign_in_expired","message":"sign-in expired, open the CLI to sign in again"}"#),
                account(
                    "cursor:1", "cursor", "Cursor", label: nil, status: "error",
                    error: #"{"kind":"invalid_response","message":"Unexpected response from cursor.com"}"#,
                    windows: [window("other:requests", 40, "good", "untracked", even: nil, spare: nil, resets: nil)]),
                account(
                    "codex:free", "codex", "Codex", label: "free", status: "no_subscription",
                    error: #"{"kind":"no_subscription","message":"No active ChatGPT subscription (Free plan)."}"#),
            ]
        }

        private static func account(
            _ id: String, _ provider: String, _ name: String, label: String?, status: String, error: String,
            windows: [String] = [], balances: String = "[]", notices: String = "[]"
        ) -> String {
            let labelJSON = label.map { "\"\($0)\"" } ?? "null"
            let plan = status == "no_subscription" ? "null" : "\"Pro\""
            return """
                {"id":"\(id)","provider":"\(provider)","provider_name":"\(name)","label":\(labelJSON),"email":null,
                "plan":\(plan),"hidden":false,"owner":"cli","status":"\(status)","error":\(error),
                "updated_at":"2026-09-23T09:58:00Z","source":"live","windows":[\(windows.joined(separator: ","))],
                "balances":\(balances),"notices":\(notices),"usage_home":"~/.\(provider)"}
                """
        }

        private static func window(
            _ id: String, _ used: Double, _ tone: String, _ severity: String, even: Double?, spare: Double?,
            resets: String?, runsOut: String? = nil
        ) -> String {
            let number = { (value: Double?) in value.map { "\($0)" } ?? "null" }
            let stamp = { (value: String?) in value.map { "\"\($0)\"" } ?? "null" }
            let label = id.split(separator: ":").last.map { String($0).capitalized } ?? id
            return """
                {"id":"\(id)","label":"\(label)","used_percent":\(used),"remaining_percent":\(100 - used),
                "resets_at":\(stamp(resets)),"period_seconds":18000,"tone":"\(tone)","hidden":false,
                "pace":{"severity":"\(severity)","even_pace_percent":\(number(even)),
                "projected_percent":\(number(spare.map { 100 - $0 })),"spare_percent":\(number(spare)),
                "runs_out_at":\(stamp(runsOut))}}
                """
        }

        private static func usage(_ provider: String, _ name: String, _ home: String, seed: Int) -> String {
            let days = (0..<30).map { index -> String in
                let tokens = (index * seed * 7_919) % 90_000 * (index % 4 == 0 ? 0 : 1)
                let date = String(format: "2026-%02d-%02d", index < 7 ? 8 : 9, index < 7 ? 25 + index : index - 6)
                return
                    #"{"date":"\#(date)","total_tokens":\#(tokens),"cost_usd_micros":\#(tokens * 40),"partial":false}"#
            }
            let totals = totalsJSON(tokens: 1_234_000 * seed, micros: 2_310_000 * Int64(seed))
            return """
                {"provider":"\(provider)","provider_name":"\(name)","usage_home":"\(home)","today":\(totals),
                "yesterday":\(totals),"last_30_days":\(totals),"daily":[\(days.joined(separator: ","))]}
                """
        }

        private static func totalsJSON(tokens: Int, micros: Int64) -> String {
            """
            {"tokens":{"input":\(tokens),"cache_read":0,"cache_write":0,"output":0,"reasoning":0,"total":\(tokens)},
            "cost_usd_micros":\(micros),"partial":false,"unpriced_tokens":0,"unpriced_models":[],
            "models":[{"model":"top-model","total_tokens":\(tokens),"cost_usd_micros":\(micros),"partial":false}],
            "models_other":null}
            """
        }

        private static func spend(singleProvider: Bool) -> String {
            let claude = providerSpend("claude", "Claude", micros: 14_370_000, tokens: 18_400_000)
            let codex = providerSpend("codex", "Codex", micros: 4_050_000, tokens: 6_120_000)
            let today = singleProvider ? [codex] : [claude, codex]
            return """
                {"today":\(period(today)),"yesterday":\(period([codex])),"last_30_days":\(period([claude, codex]))}
                """
        }

        private struct SpendEntry {
            let json: String
            let micros: Int64
            let tokens: Int
        }

        private static func providerSpend(_ provider: String, _ name: String, micros: Int64, tokens: Int) -> SpendEntry
        {
            let json = """
                {"provider":"\(provider)","provider_name":"\(name)","cost_usd_micros":\(micros),"total_tokens":\(tokens),
                "partial":false,"models":[{"model":"\(provider)-large","total_tokens":\(tokens / 2),
                "cost_usd_micros":\(micros / 2),"partial":false},{"model":"\(provider)-small","total_tokens":\(tokens / 2),
                "cost_usd_micros":\(micros / 2),"partial":false}],"models_other":null}
                """
            return SpendEntry(json: json, micros: micros, tokens: tokens)
        }

        private static func period(_ entries: [SpendEntry]) -> String {
            let micros = entries.map(\.micros).reduce(0, +)
            let tokens = entries.map(\.tokens).reduce(0, +)
            let providers = entries.map(\.json).joined(separator: ",")
            return """
                {"cost_usd_micros":\(micros),"total_tokens":\(tokens),"partial":false,"by_provider":[\(providers)]}
                """
        }
    }
#endif
