import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationSpend06Tests: XCTestCase {
    private func card(
        _ unit: SpendUnit = .cost, breakdown: SpendBreakdown = .models, period: SpendPeriodPreference = .last30Days,
        spend: Spend? = nil, formatter: DisplayFormatter = Build.english
    ) throws -> SpendCardModel {
        let source = try spend ?? SpendSamples.spend(SpendSamples.twoProviders())
        return SpendCardModel.make(
            spend: source, selection: SpendSelection(period: period, unit: unit, breakdown: breakdown),
            formatter: formatter)
    }

    func testUnitsChangeCenterLegendAndRing() throws {
        let cost = try card()
        XCTAssertEqual(cost.title, "Total Spend")
        XCTAssertEqual(cost.centerAmount, "$115")
        XCTAssertNil(cost.centerCaption)
        XCTAssertEqual(cost.entries.map(\.amount), ["$66.45", "$48.90"])
        XCTAssertEqual(cost.fractions[0], 66_450_000.0 / 115_350_000, accuracy: 1e-9)

        let tokens = try card(.tokens)
        XCTAssertEqual(tokens.title, "Total Tokens")
        XCTAssertEqual(tokens.centerAmount, "196M")
        XCTAssertEqual(tokens.centerCaption, "tokens")
        XCTAssertEqual(tokens.entries.map(\.amount), ["124M", "71.8M"])
        XCTAssertEqual(tokens.fractions[0], 124_100_000.0 / 195_900_000, accuracy: 1e-9)

        let rate = try card(.costPerMTok)
        XCTAssertEqual(rate.title, "Cost per MTok")
        XCTAssertEqual(rate.centerAmount, "$0.59")
        XCTAssertEqual(rate.centerCaption, "blended")
        XCTAssertEqual(rate.entries.map(\.amount), ["$0.54", "$0.68"])
        XCTAssertEqual(rate.fractions, tokens.fractions)
        XCTAssertNil(rate.entries[0].tokensLine)
    }

    func testMissingSevenDaysFallsBackToThirtyDays() throws {
        let spend = try SpendSamples.spend(SpendSamples.twoProviders())
        XCTAssertEqual(try card(period: .last7Days, spend: spend).period, .last30Days)
        XCTAssertEqual(SpendPeriodPreference.available(in: spend), [.today, .yesterday, .last30Days])
        let week = try SpendSamples.spend(SpendSamples.twoProviders(), last7Days: SpendSamples.twoProviders())
        XCTAssertEqual(try card(period: .last7Days, spend: week).period, .last7Days)
        XCTAssertEqual(
            SpendPeriodPreference.allCases.map { $0.title(Build.russian.strings) },
            ["Сегодня", "Вчера", "7 дней", "30 дней"])
    }

    func testLegendHoverOpensModelPopover() throws {
        guard case .models(let popover) = try card().entries.first?.tip else { return XCTFail("expected popover") }
        XCTAssertEqual(popover.title, "Claude · Last 30 Days")
        XCTAssertEqual(popover.total, "$66.45")
        XCTAssertEqual(popover.rows.map(\.name), ["Opus 4.6", "Sonnet 4.6", "Haiku 4.5", "Other"])
        XCTAssertEqual(popover.rows.last?.detail, "· 2 models")
        XCTAssertEqual(
            popover.rows.map(\.figures),
            ["62% · 38.4M tokens", "29% · 52.1M tokens", "5% · 31.7M tokens", "2% · 1.9M tokens"])
        XCTAssertEqual(popover.rows[0].fill, 41_200_000.0 / 66_450_000, accuracy: 1e-9)
        XCTAssertEqual(
            popover.footnotes,
            ["Models under 5% are folded into Other.", "Estimated from local logs and public pricing."])
    }

    func testModelsBreakdownMergesProvidersAndFoldsSmallShares() throws {
        let breakdown = try XCTUnwrap(try card().breakdown)
        XCTAssertEqual(breakdown.modes, [.models, .projects])
        XCTAssertTrue(breakdown.showsSwitch)
        XCTAssertEqual(breakdown.caption, "7 models")
        XCTAssertEqual(breakdown.rows.map(\.name), ["Opus 4.6", "GPT-5-Codex", "Sonnet 4.6", "GPT-5", "Other"])
        XCTAssertEqual(breakdown.rows.map(\.share), ["35.7%", "33.4%", "17.1%", "8.9%", "4.7%"])
        XCTAssertEqual(breakdown.rows.last?.detail, "· 3 models")
        XCTAssertEqual(breakdown.rows.last?.value, "$5.45")
        XCTAssertEqual(breakdown.rows.last?.segments.map(\.id), ["claude"])
        XCTAssertEqual(breakdown.rows[1].icon, .dot(ProviderStyle.seriesColor(for: "codex")))
    }

    func testProjectsBreakdownSplitsBarsByProvider() throws {
        let breakdown = try XCTUnwrap(try card(breakdown: .projects).breakdown)
        XCTAssertEqual(breakdown.mode, .projects)
        XCTAssertEqual(breakdown.caption, "9 projects")
        XCTAssertEqual(breakdown.rows.map(\.name), ["~/work/headroom", "No project", "Other"])
        XCTAssertEqual(breakdown.rows.map(\.detail), [nil, nil, "· 7 projects"])
        XCTAssertEqual(breakdown.rows.map(\.share), ["41.7%", "23.7%", "34.5%"])
        XCTAssertEqual(breakdown.rows[0].segments.map(\.id), ["claude", "codex"])
        XCTAssertEqual(breakdown.rows[0].segments[1].fraction, 18_200_000.0 / 115_350_000, accuracy: 1e-9)
        XCTAssertEqual(breakdown.rows.map(\.icon), [.folder, .folder, .folder])
        let tokens = try XCTUnwrap(try card(.tokens, breakdown: .projects).breakdown)
        XCTAssertEqual(tokens.rows.map(\.value), ["90M", "40M", "65.9M"])
        XCTAssertEqual(tokens.rows.map(\.share), ["45.9%", "20.4%", "33.6%"])
    }

    func testBreakdownHiddenWithoutProjectsAndSwitchHiddenWhenEmpty() throws {
        let legacy = try SpendSamples.spend(SpendSamples.twoProviders(projects: nil))
        XCTAssertNil(try card(spend: legacy).breakdown)
        let empty = try SpendSamples.spend(SpendSamples.twoProviders(projects: "[]", other: "null"))
        let breakdown = try XCTUnwrap(try card(breakdown: .projects, spend: empty).breakdown)
        XCTAssertEqual(breakdown.modes, [.models])
        XCTAssertEqual(breakdown.mode, .models)
        XCTAssertFalse(breakdown.showsSwitch)
    }

    func testRussianBreakdownUsesCommaAndPlurals() throws {
        let breakdown = try XCTUnwrap(try card(breakdown: .projects, formatter: Build.russian).breakdown)
        XCTAssertEqual(breakdown.caption, "9 проектов")
        XCTAssertEqual(breakdown.rows.map(\.share), ["41,7%", "23,7%", "34,5%"])
        XCTAssertEqual(breakdown.rows.last?.name, "Другие")
    }

    func testSelectionSeedsFromSettingsOnlyOnRelease06() throws {
        var display = try Build.display()
        display.spendPeriod = .last7Days
        display.spendUnit = .tokens
        XCTAssertEqual(SpendSelection.seed(display: display, features: .legacy), .legacyDefault)
        XCTAssertEqual(
            SpendSelection.seed(display: display, features: .current),
            SpendSelection(period: .last7Days, unit: .tokens, breakdown: .models))
        XCTAssertEqual(SpendCardModel.units(.legacy), [.cost])
        XCTAssertEqual(SpendCardModel.units(.current), [.cost, .tokens, .costPerMTok])
        let old = SpendSelection.legacyDefault
        var new = old
        new.unit = .costPerMTok
        new.breakdown = .projects
        XCTAssertEqual(
            SpendSelection.change(from: old, to: new), [.spend(.unit(.costPerMTok)), .spend(.breakdown(.projects))])
    }

    func testShareMath() {
        XCTAssertEqual(SpendShare.permille(1, of: 3), 333)
        XCTAssertEqual(SpendShare.permille(5, of: 3), 1000)
        XCTAssertEqual(SpendShare.permille(1, of: 0), 0)
        XCTAssertEqual(SpendShare.permille(UInt64.max / 2, of: UInt64.max), 499)
        XCTAssertEqual(SpendShare.decimalPercent(357, language: .en), "35.7%")
        XCTAssertEqual(SpendShare.wholePercent(619), "61%")
    }
}
