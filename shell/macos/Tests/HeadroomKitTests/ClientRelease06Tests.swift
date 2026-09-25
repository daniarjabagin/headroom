import Foundation
import XCTest

@testable import HeadroomKit

@MainActor
final class ClientRelease06Tests: XCTestCase {
    private nonisolated static let spendReport = #"""
        {"since":"2026-09-17","until":"2026-09-23","by":"model",
         "rows":[{"key":"claude-opus-4-5","provider":"claude",
           "tokens":{"input":1200000,"cache_read":180000000,"cache_write":9000000,"output":2100000,"reasoning":0,
             "total":192300000},
           "cost_usd_micros":151200000,"partial":false,"unpriced_tokens":0,"cost_per_mtok_usd_micros":786271,
           "share_permille":767}],
         "total":{"key":null,"provider":null,
           "tokens":{"input":1200000,"cache_read":180000000,"cache_write":9000000,"output":2100000,"reasoning":0,
             "total":192300000},
           "cost_usd_micros":197100000,"partial":true,"unpriced_tokens":5000,"cost_per_mtok_usd_micros":null,
           "share_permille":1000}}
        """#

    private nonisolated static let diagnostics = #"""
        {"app_version":"0.6.0","os":"macOS 27.0","desktop":null,"uptime_secs":7530,"transports":["socket"],
         "log_level":"info","log_level_source":"settings","log_file":"~/Library/Logs/Headroom/headroom.log",
         "providers":[{"provider":"claude","accounts":1,"usage_homes":1}],"accounts":[],
         "text":"Headroom 0.6.0\nOS: macOS 27.0\n"}
        """#

    private func connected(settings: String) async throws -> (DaemonClient, FakeTransport) {
        let state = try Fixture.text("state_empty")
        let transport = FakeTransport { request in
            switch request.method {
            case "GetState": [Reply.result(request.id, state)]
            case "GetSettings": [Reply.result(request.id, settings)]
            case "GetSpend": [Reply.result(request.id, Self.spendReport)]
            case "GetDiagnostics": [Reply.result(request.id, Self.diagnostics)]
            default: [Reply.result(request.id, "null")]
            }
        }
        let client = DaemonClient(transport: transport, sleeper: RecordingSleeper())
        await client.start()
        _ = await collect(client.events) { $0.contains { if case .state = $0 { true } else { false } } }
        return (client, transport)
    }

    private func failure(_ outcome: CommandQueue.Outcome) -> DaemonError? {
        guard case .failure(let error) = outcome else { return nil }
        return error
    }

    private func sent(_ transport: FakeTransport) throws -> [FakeRequest] {
        Array(try XCTUnwrap(transport.connections.first).requests.dropFirst(2))
    }

    func testGetSpendSendsTheQueryAsAStringAndDecodesTheReport() async throws {
        let (client, transport) = try await connected(settings: SettingsTests.release06)
        let report = try await client.getSpend(
            SpendQuery(range: .days(since: "2026-09-17", until: nil), grouping: .model, provider: "claude"))
        XCTAssertEqual(report.by, .model)
        XCTAssertEqual(report.rows.first?.key, "claude-opus-4-5")
        XCTAssertEqual(report.rows.first?.tokens.total, 192_300_000)
        XCTAssertEqual(report.rows.first?.costPerMTokUSDMicros, 786_271)
        XCTAssertNil(report.total.key)
        XCTAssertNil(report.total.costPerMTokUSDMicros)
        XCTAssertEqual(report.total.unpricedTokens, 5000)
        _ = try await client.getSpend(SpendQuery(range: .period(.last7Days), grouping: .project))
        let requests = try sent(transport)
        XCTAssertEqual(requests.map(\.method), ["GetSpend", "GetSpend"])
        XCTAssertEqual(requests[0].params, [.string(#"{"by":"model","provider":"claude","since":"2026-09-17"}"#)])
        XCTAssertEqual(requests[1].params, [.string(#"{"by":"project","period":"7d"}"#)])
        await client.stop()
    }

    func testSpendQueryEncodesAnUntilDay() throws {
        let query = SpendQuery(range: .days(since: "2026-09-01", until: "2026-09-10"), grouping: .day)
        XCTAssertEqual(try RPCCodec.encodeString(query), #"{"by":"day","since":"2026-09-01","until":"2026-09-10"}"#)
    }

    func testDiagnosticsDecode() async throws {
        let (client, transport) = try await connected(settings: SettingsTests.release06)
        let store = SettingsStore(client: client, queue: CommandQueue(client: client))
        let report = try await store.diagnostics()
        XCTAssertEqual(report.appVersion, "0.6.0")
        XCTAssertEqual(report.os, "macOS 27.0")
        XCTAssertNil(report.desktop)
        XCTAssertEqual(report.uptimeSecs, 7530)
        XCTAssertEqual(report.transports, ["socket"])
        XCTAssertEqual(report.logFile, "~/Library/Logs/Headroom/headroom.log")
        XCTAssertEqual(report.text, "Headroom 0.6.0\nOS: macOS 27.0\n")
        XCTAssertEqual(try sent(transport).map(\.method), ["GetDiagnostics"])
        await client.stop()
    }

    func testRelease06ChangesAreSentOnlyToARelease06Service() async throws {
        let (client, transport) = try await connected(settings: SettingsTests.legacy)
        let store = SettingsStore(client: client, queue: CommandQueue(client: client))
        await store.reload()
        XCTAssertEqual(store.features, .legacy)
        let refused = await store.change(.density(.compact)).value
        XCTAssertEqual(failure(refused), SettingsStore.needsRelease06)
        XCTAssertEqual(store.settings?.display.density, .normal)
        let reset = await store.resetAll().value
        XCTAssertEqual(failure(reset), SettingsStore.needsRelease06)
        try await store.change(.theme(.dark)).value.get()
        XCTAssertEqual(
            try sent(transport).map(\.method), ["GetSettings", "ListProviders", "UpdateSettings", "GetSettings"])
        await client.stop()
    }

    func testRelease06ServiceGetsNewKeysAndResets() async throws {
        let (client, transport) = try await connected(settings: SettingsTests.release06)
        let store = SettingsStore(client: client, queue: CommandQueue(client: client))
        await store.reload()
        XCTAssertEqual(store.features, .current)
        let write = store.change(.density(.normal))
        XCTAssertEqual(store.settings?.display.density, .normal)
        try await write.value.get()
        try await store.resetAll().value.get()
        let requests = try sent(transport)
        XCTAssertEqual(
            requests.map(\.method),
            ["GetSettings", "ListProviders", "UpdateSettings", "GetSettings", "ResetSettings", "GetSettings"])
        XCTAssertEqual(requests[2].params, [.string(#"{"display":{"density":"normal"}}"#)])
        XCTAssertEqual(requests[4].params, [])
        await client.stop()
    }

    func testInvalidResultsAreRefusedLocally() async throws {
        let (client, transport) = try await connected(settings: SettingsTests.release06)
        let store = SettingsStore(client: client, queue: CommandQueue(client: client))
        await store.reload()
        let outcome = await store.change(.shortcut("Ctrl+U")).value
        XCTAssertEqual(failure(outcome), .invalidArguments(#"Invalid setting: invalidShortcut("Ctrl+U")"#))
        XCTAssertEqual(store.settings?.shortcuts.open, "<Super>u")
        XCTAssertNil(store.lastError)
        XCTAssertEqual(try sent(transport).map(\.method), ["GetSettings", "ListProviders"])
        await client.stop()
    }
}
