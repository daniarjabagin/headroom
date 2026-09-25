import Foundation

@testable import HeadroomKit

enum SpendSamples {
    static func model(_ name: String, cost: Int64, tokens: UInt64, rate: Int64? = nil) -> String {
        """
        {"model":"\(name)","total_tokens":\(tokens),"cost_usd_micros":\(cost),"partial":false,
         "cost_per_mtok_usd_micros":\(rate.map(String.init) ?? "null")}
        """
    }

    static func provider(
        _ id: String, cost: Int64, tokens: UInt64, models: [String], other: String = "null", rate: Int64? = nil
    ) -> String {
        """
        {"provider":"\(id)","provider_name":"\(id.capitalized)","cost_usd_micros":\(cost),"total_tokens":\(tokens),
         "partial":false,"models":[\(models.joined(separator: ","))],"models_other":\(other),
         "cost_per_mtok_usd_micros":\(rate.map(String.init) ?? "null")}
        """
    }

    static func period(
        cost: Int64, tokens: UInt64, providers: [String], projects: String? = "[]", projectsOther: String = "null",
        rate: Int64? = nil
    ) -> String {
        let projectsField = projects.map { #","projects":\#($0),"projects_other":\#(projectsOther)"# } ?? ""
        return """
            {"cost_usd_micros":\(cost),"total_tokens":\(tokens),"partial":false,
             "cost_per_mtok_usd_micros":\(rate.map(String.init) ?? "null"),
             "by_provider":[\(providers.joined(separator: ","))]\(projectsField)}
            """
    }

    static func spend(_ period: String, last7Days: String? = nil) throws -> Spend {
        let week = last7Days.map { #""last_7_days":\#($0),"# } ?? ""
        return try Fixture.decode(
            Spend.self, json: #"{"today":\#(period),"yesterday":\#(period),\#(week)"last_30_days":\#(period)}"#)
    }

    static let claude = provider(
        "claude", cost: 66_450_000, tokens: 124_100_000,
        models: [
            model("Opus 4.6", cost: 41_200_000, tokens: 38_400_000, rate: 1_072_917),
            model("Sonnet 4.6", cost: 19_800_000, tokens: 52_100_000),
            model("Haiku 4.5", cost: 3_960_000, tokens: 31_700_000),
        ],
        other: #"{"count":2,"total_tokens":1900000,"cost_usd_micros":1490000,"partial":false}"#, rate: 535_455)

    static let codex = provider(
        "codex", cost: 48_900_000, tokens: 71_800_000,
        models: [
            model("GPT-5-Codex", cost: 38_600_000, tokens: 60_000_000),
            model("GPT-5", cost: 10_300_000, tokens: 11_800_000),
        ], rate: 681_058)

    static let projects = #"""
        [{"project":"~/work/headroom","cost_usd_micros":48200000,"total_tokens":90000000,"partial":false,
          "share_permille":417,"by_provider":[
            {"provider":"claude","provider_name":"Claude","cost_usd_micros":30000000,"total_tokens":60000000},
            {"provider":"codex","provider_name":"Codex","cost_usd_micros":18200000,"total_tokens":30000000}]},
         {"project":null,"cost_usd_micros":27350000,"total_tokens":40000000,"partial":false,"share_permille":236,
          "by_provider":[
            {"provider":"claude","provider_name":"Claude","cost_usd_micros":27350000,"total_tokens":40000000}]}]
        """#

    static let projectsOther =
        #"{"count":7,"cost_usd_micros":39800000,"total_tokens":65900000,"partial":false,"share_permille":345}"#

    static func twoProviders(projects: String? = SpendSamples.projects, other: String = SpendSamples.projectsOther)
        -> String
    {
        period(
            cost: 115_350_000, tokens: 195_900_000, providers: [claude, codex], projects: projects,
            projectsOther: other, rate: 588_821)
    }
}
