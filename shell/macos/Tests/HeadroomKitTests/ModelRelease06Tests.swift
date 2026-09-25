import Foundation
import XCTest

@testable import HeadroomKit

final class ModelRelease06Tests: XCTestCase {
    func testPanelItemsAndToneFromTheFullSnapshot() throws {
        let state = try Fixture.decode(DaemonState.self, "state_full")
        XCTAssertTrue(state.reportsPanelItems)
        XCTAssertEqual(state.panelTone, .critical)
        let item = try XCTUnwrap(state.panelItems.first)
        XCTAssertEqual(state.panelItems.count, 1)
        XCTAssertEqual(item.accountID, "claude:main")
        XCTAssertEqual(item.window, "session")
        XCTAssertEqual(item.valuePercent, 8.0)
        XCTAssertEqual(item.evenPacePercent, 90.0)
        XCTAssertEqual(item.logo, "claude")
        XCTAssertEqual(item.tone, .critical)
        XCTAssertFalse(item.combined)
        XCTAssertEqual(item.accountCount, 1)
        XCTAssertEqual(item.id, "account/claude:main/session")
    }

    func testEmptySnapshotReportsEmptyPanelAndNoTone() throws {
        let state = try Fixture.decode(DaemonState.self, "state_empty")
        XCTAssertTrue(state.reportsPanelItems)
        XCTAssertEqual(state.panelItems, [])
        XCTAssertNil(state.panelTone)
        XCTAssertEqual(state.providerStatus, [])
        XCTAssertEqual(state.spend.last7Days?.projects, [])
        XCTAssertNil(state.spend.last7Days?.projectsOther)
        XCTAssertNil(state.spend.today.costPerMTokUSDMicros)
    }

    func testCombinedSnapshotPanelItemIsCombined() throws {
        let state = try Fixture.decode(DaemonState.self, "state_combined")
        XCTAssertEqual(state.panelItems.count, 1)
        XCTAssertEqual(state.panelItems.first?.combined, state.headline?.combined)
        XCTAssertEqual(state.combined.map(\.collapsed), [false])
        XCTAssertTrue(state.accounts.allSatisfy { !$0.collapsed })
    }

    func testAccountRefreshAndCollapse() throws {
        let state = try Fixture.decode(DaemonState.self, "state_full")
        let refresh = try XCTUnwrap(state.accounts.first?.refresh)
        XCTAssertEqual(refresh.mode, .idle)
        XCTAssertEqual(refresh.intervalSecs, 300)
        XCTAssertEqual(refresh.nextAt, try Fixture.timestamp("2026-09-23T10:03:00Z"))
        XCTAssertEqual(refresh.reason, .schedule)
        XCTAssertFalse(try XCTUnwrap(state.accounts.first).collapsed)
        let live = try Fixture.decode(
            AccountRefresh.self,
            json: #"{"mode":"live","interval_secs":60,"next_at":null,"reason":"activity"}"#)
        XCTAssertEqual(live.mode, .live)
        XCTAssertNil(live.nextAt)
        XCTAssertEqual(live.reason, .activity)
        let future = try Fixture.decode(AccountRefresh.self, json: #"{"mode":"turbo","interval_secs":60}"#)
        XCTAssertEqual(future.mode, .idle)
        XCTAssertEqual(future.reason, .unknown)
    }

    func testProviderStatus() throws {
        let state = try Fixture.decode(DaemonState.self, "state_full")
        let status = try XCTUnwrap(state.providerStatus.first)
        XCTAssertEqual(status.provider, "claude")
        XCTAssertEqual(status.indicator, .minor)
        XCTAssertEqual(status.tone, .warning)
        XCTAssertEqual(status.title, "Elevated errors on Claude Code")
        XCTAssertEqual(status.stage, "identified")
        XCTAssertEqual(status.startedAt, try Fixture.timestamp("2026-09-23T09:12:00Z"))
        XCTAssertEqual(status.url, "https://stspg.io/abc123")
        XCTAssertFalse(status.isClear)
        let clear = try Fixture.decode(
            ProviderStatus.self,
            json: #"{"provider":"codex","indicator":"none","tone":"neutral","title":null,"stage":null,"#
                + #""started_at":null,"url":"https://status.openai.com"}"#)
        XCTAssertTrue(clear.isClear)
        XCTAssertNil(clear.title)
    }

    func testSpendAdditionsFromTheFullSnapshot() throws {
        let spend = try Fixture.decode(DaemonState.self, "state_full").spend
        let week = try XCTUnwrap(spend.last7Days)
        XCTAssertEqual(week.costPerMTokUSDMicros, 2_000_000)
        XCTAssertEqual(week.byProvider.first?.costPerMTokUSDMicros, 2_000_000)
        XCTAssertEqual(week.byProvider.first?.models.first?.costPerMTokUSDMicros, 2_000_000)
        let project = try XCTUnwrap(week.projects?.first)
        XCTAssertNil(project.project)
        XCTAssertEqual(project.sharePermille, 1000)
        XCTAssertEqual(project.byProvider.map(\.provider), ["claude", "codex"])
        XCTAssertEqual(project.byProvider.map(\.costUSDMicros).reduce(0, +), project.costUSDMicros)
        XCTAssertEqual(spend.period(.last7Days), week)
        XCTAssertEqual(spend.period(.today), spend.today)
        XCTAssertEqual(SpendPeriodPreference.available(in: spend), SpendPeriodPreference.allCases)
    }

    func testProjectsOtherAndPerProjectProviders() throws {
        let period = try Fixture.decode(PeriodSpend.self, json: StateSamples.spendPeriod)
        XCTAssertEqual(period.costPerMTokUSDMicros, 217_163)
        XCTAssertEqual(period.projects?.first?.project, "~/code/headroom")
        XCTAssertEqual(period.projects?.first?.byProvider.last?.totalTokens, 8_100_000)
        let other = try XCTUnwrap(period.projectsOther)
        XCTAssertEqual(other.count, 4)
        XCTAssertEqual(other.sharePermille, 266)
        XCTAssertTrue(other.partial)
        let listed = (period.projects ?? []).map(\.costUSDMicros).reduce(0, +)
        XCTAssertEqual(listed + other.costUSDMicros, period.costUSDMicros)
    }

    func testLegacyStateKeepsWorking() throws {
        let state = try Fixture.decode(DaemonState.self, json: StateSamples.legacy)
        XCTAssertFalse(state.reportsPanelItems)
        XCTAssertNil(state.panelTone)
        XCTAssertEqual(state.providerStatus, [])
        let item = try XCTUnwrap(state.panelItems.first)
        XCTAssertEqual(state.panelItems.count, 1)
        XCTAssertEqual(item.accountID, "codex:work")
        XCTAssertEqual(item.valuePercent, 55.0)
        XCTAssertNil(item.evenPacePercent)
        XCTAssertEqual(item.logo, "codex")
        XCTAssertEqual(item.accountCount, 1)
        XCTAssertFalse(state.accounts[0].collapsed)
        XCTAssertNil(state.accounts[0].refresh)
        XCTAssertFalse(state.combined[0].collapsed)
        XCTAssertNil(state.spend.last7Days)
        XCTAssertNil(state.spend.today.projects)
        XCTAssertNil(state.spend.today.costPerMTokUSDMicros)
        XCTAssertEqual(SpendPeriodPreference.available(in: state.spend), [.today, .yesterday, .last30Days])
        XCTAssertEqual(state.display.timeFormat, .auto)
    }

    func testProviderLinksKeepOnlyHTTPS() throws {
        let providers = try Fixture.decode(ProvidersPayload.self, "providers").providers
        let codex = try XCTUnwrap(providers.first { $0.id == "codex" })
        XCTAssertEqual(codex.links?.status?.absoluteString, "https://status.openai.com")
        XCTAssertEqual(codex.links?.url(.dashboard)?.absoluteString, "https://chatgpt.com/codex")
        XCTAssertNil(codex.links?.usage)
        XCTAssertTrue(providers.allSatisfy { $0.links != nil })
        let links = try Fixture.decode(
            ProviderLinks.self,
            json: #"{"status":"http://status.example","dashboard":"javascript:alert(1)","usage":"https://x.example/u"}"#
        )
        XCTAssertNil(links.status)
        XCTAssertNil(links.dashboard)
        XCTAssertEqual(links.usage?.host, "x.example")
        let legacy = try Fixture.decode(
            ProviderInfo.self,
            json: #"{"id":"codex","display_name":"Codex","add_account":[],"multi_account":true,"local_usage":true}"#)
        XCTAssertNil(legacy.links)
    }
}
