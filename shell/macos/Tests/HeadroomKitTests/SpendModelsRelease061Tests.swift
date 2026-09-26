import Foundation
import XCTest

@testable import HeadroomKit

final class SpendModelsRelease061Tests: XCTestCase {
    private static func merged(
        _ name: String, provider: String, cost: Int64, tokens: UInt64, rate: Int64?, partial: Bool = false
    ) -> String {
        """
        {"provider":"\(provider)","provider_name":"\(provider.capitalized)","model":"\(name)",
         "total_tokens":\(tokens),"cost_usd_micros":\(cost),"partial":\(partial),
         "cost_per_mtok_usd_micros":\(rate.map(String.init) ?? "null")}
        """
    }

    private static let mergedModels = [
        merged("Opus 4.6", provider: "claude", cost: 41_200_000, tokens: 38_400_000, rate: 1_072_917),
        merged("GPT-5-Codex", provider: "codex", cost: 38_600_000, tokens: 60_000_000, rate: 643_333),
        merged("Sonnet 4.6", provider: "claude", cost: 19_800_000, tokens: 52_100_000, rate: 380_038),
        merged("GPT-5", provider: "codex", cost: 10_300_000, tokens: 11_800_000, rate: 872_881),
        merged("Haiku 4.5", provider: "claude", cost: 3_960_000, tokens: 31_700_000, rate: 124_921),
    ]

    private static func withModels(models: [String], other: String) -> String {
        let period = SpendSamples.twoProviders()
        let fields = #"{"models":[\#(models.joined(separator: ","))],"models_other":\#(other),"#
        return fields + String(period.dropFirst())
    }

    private static let otherRated =
        #"{"count":2,"total_tokens":1900000,"cost_usd_micros":1490000,"partial":false,"cost_per_mtok_usd_micros":784211}"#
    private static let otherUnrated =
        #"{"count":2,"total_tokens":1900000,"cost_usd_micros":1490000,"partial":true,"cost_per_mtok_usd_micros":null}"#

    private func breakdown(_ unit: SpendUnit, other: String = otherRated) throws -> SpendBreakdownModel {
        let spend = try SpendSamples.spend(Self.withModels(models: Self.mergedModels, other: other))
        let card = SpendCardModel.make(
            spend: spend, selection: SpendSelection(period: .last30Days, unit: unit, breakdown: .models),
            formatter: Build.english)
        return try XCTUnwrap(card.breakdown)
    }

    func testPeriodDecodesTheDaemonModelsList() throws {
        let spend = try SpendSamples.spend(Self.withModels(models: Self.mergedModels, other: Self.otherRated))
        let models = try XCTUnwrap(spend.last30Days.models)
        XCTAssertEqual(models.count, 5)
        XCTAssertEqual(models[1].provider, "codex")
        XCTAssertEqual(models[1].providerName, "Codex")
        XCTAssertEqual(models[1].model, "GPT-5-Codex")
        XCTAssertEqual(models[1].costPerMTokUSDMicros, 643_333)
        XCTAssertEqual(spend.last30Days.modelsOther?.costPerMTokUSDMicros, 784_211)
        let old = try SpendSamples.spend(SpendSamples.twoProviders())
        XCTAssertNil(old.last30Days.models)
        XCTAssertNil(old.last30Days.modelsOther)
    }

    func testSnapshotCarriesTheMergedModels() throws {
        let state = try Fixture.decode(DaemonState.self, "state_full")
        let today = try XCTUnwrap(state.spend.today.models)
        XCTAssertEqual(today.map(\.provider), ["claude", "codex"])
        XCTAssertEqual(today.map(\.model), ["claude-opus", "gpt-5.5"])
        XCTAssertEqual(today.map(\.costUSDMicros).reduce(0, +), state.spend.today.costUSDMicros)
        XCTAssertNil(state.spend.today.modelsOther)
        let yesterday = try XCTUnwrap(state.spend.yesterday.models)
        XCTAssertEqual(yesterday.map(\.model), ["gpt-5.5", "unknown"])
        XCTAssertEqual(yesterday.map(\.partial), [false, true])
    }

    func testDaemonListIsRenderedAsGiven() throws {
        let list = try breakdown(.cost)
        XCTAssertEqual(list.caption, "7 models")
        XCTAssertEqual(list.rows.map(\.name), ["Opus 4.6", "GPT-5-Codex", "Sonnet 4.6", "GPT-5", "Haiku 4.5", "Other"])
        XCTAssertEqual(list.rows.map(\.share), ["35.7%", "33.4%", "17.1%", "8.9%", "3.4%", "1.2%"])
        XCTAssertEqual(list.rows.map(\.value), ["$41.20", "$38.60", "$19.80", "$10.30", "$3.96", "$1.49"])
        XCTAssertEqual(list.rows.last?.detail, "· 2 models")
        XCTAssertEqual(list.rows.last?.id, "model:other")
        XCTAssertEqual(list.rows.last?.icon, .dot(SpendBreakdownModel.neutral))
        XCTAssertEqual(list.rows.last?.segments.map(\.id), ["other"])
        XCTAssertEqual(list.rows[1].icon, .dot(ProviderStyle.seriesColor(for: "codex")))
        XCTAssertEqual(list.rows[1].id, "model:codex:GPT-5-Codex")
    }

    func testCostPerMTokShowsTheDaemonRates() throws {
        let rated = try breakdown(.costPerMTok)
        XCTAssertEqual(rated.rows.map(\.value), ["$1.07", "$0.64", "$0.38", "$0.87", "$0.12", "$0.78"])
        let unrated = try breakdown(.costPerMTok, other: Self.otherUnrated)
        XCTAssertEqual(unrated.rows.last?.value, "—")
    }

    func testTokensUnitSharesByTokens() throws {
        let list = try breakdown(.tokens)
        XCTAssertEqual(list.rows.map(\.value), ["38.4M", "60M", "52.1M", "11.8M", "31.7M", "1.9M"])
        XCTAssertEqual(list.rows.first?.share, "19.6%")
    }

    func testEmptyDaemonListFallsBackToProjectsOnly() throws {
        let spend = try SpendSamples.spend(Self.withModels(models: [], other: "null"))
        let card = SpendCardModel.make(
            spend: spend, selection: SpendSelection(period: .last30Days, unit: .cost, breakdown: .models),
            formatter: Build.english)
        let breakdown = try XCTUnwrap(card.breakdown)
        XCTAssertEqual(breakdown.modes, [.projects])
        XCTAssertEqual(breakdown.mode, .projects)
    }
}
