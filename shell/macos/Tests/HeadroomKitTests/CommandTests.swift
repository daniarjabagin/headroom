import Foundation
import XCTest

@testable import HeadroomKit

@MainActor
final class CommandTests: XCTestCase {
    private nonisolated static let settingsJSON = #"""
        {"refresh_interval_secs": 300,
         "notifications": {"almost_out": true, "cutting_it_close": true, "will_run_out": true, "reset": false},
         "headline": {"mode": "auto"}, "reduced_motion": false,
         "display": {"theme": "system", "language": "system", "value_mode": "left", "reset_format": "countdown",
           "panel_label": "percent", "show_spend": true, "show_account_spend": true, "show_trend": true,
           "show_forecast": true, "translucent": false, "hidden_windows": {}}}
        """#

    private func connectedClient(
        failing method: String? = nil
    ) async throws -> (DaemonClient, FakeTransport) {
        let state = try Fixture.text("state_empty")
        let providers = try Fixture.text("providers")
        let transport = FakeTransport { request in
            switch request.method {
            case method: [Reply.error(request.id, code: -32602, message: "rejected")]
            case "GetState": [Reply.result(request.id, state)]
            case "GetSettings": [Reply.result(request.id, Self.settingsJSON)]
            case "ListProviders": [Reply.result(request.id, providers)]
            default: [Reply.result(request.id, "null")]
            }
        }
        let client = DaemonClient(transport: transport, sleeper: RecordingSleeper())
        await client.start()
        _ = await collect(client.events) { $0.contains { if case .state = $0 { true } else { false } } }
        return (client, transport)
    }

    private func sent(_ transport: FakeTransport) throws -> [FakeRequest] {
        Array(try XCTUnwrap(transport.connections.first).requests.dropFirst(2))
    }

    func testQueueSendsCommandsInOrder() async throws {
        let (client, transport) = try await connectedClient()
        let queue = CommandQueue(client: client)
        queue.enqueue(.setAccountLabel(accountID: "codex:a", label: "Work"))
        queue.enqueue(.setAccountHidden(accountID: "codex:a", hidden: true))
        queue.enqueue(.restoreAccounts(provider: "codex"))
        let last = await queue.enqueue(.refresh(accountID: "codex:a")).value
        XCTAssertNoThrow(try last.get())
        let requests = try sent(transport)
        XCTAssertEqual(requests.map(\.method), ["SetAccountLabel", "SetAccountHidden", "RestoreAccounts", "Refresh"])
        XCTAssertEqual(requests[1].params, [.string("codex:a"), .bool(true)])
        XCTAssertEqual(requests[2].params, [.string("codex")])
        await client.stop()
    }

    func testModelCommandsReachTheDaemonAndRecordFailures() async throws {
        let (client, transport) = try await connectedClient(failing: "RefreshNow")
        let model = AppModel(preferredLanguages: ["en"], commands: CommandQueue(client: client))
        model.refresh(accountID: "claude:main")
        let outcome = await model.send(.refreshNow)?.value
        guard case .some(.failure(let error)) = outcome else { return XCTFail("expected failure") }
        XCTAssertEqual(error, .invalidArguments("rejected"))
        for _ in 0..<50 where model.lastError == nil { await Task.yield() }
        XCTAssertEqual(model.lastError, .invalidArguments("rejected"))
        XCTAssertEqual(try sent(transport).map(\.method), ["Refresh", "RefreshNow"])
        await client.stop()
    }

    func testPresenterRoutes() {
        let model = AppModel(preferredLanguages: ["en"])
        var routes: [SettingsRoute] = []
        model.settingsPresenter = { routes.append($0) }
        model.openSettings()
        model.signIn(provider: "claude")
        XCTAssertEqual(routes, [.general, .addAccount(provider: "claude")])
        model.refreshNow()
    }

    func testAccountOrderIsAppliedUntilTheDaemonAnswers() async throws {
        let (client, transport) = try await connectedClient()
        let model = AppModel(preferredLanguages: ["en"], commands: CommandQueue(client: client))
        let full = try Fixture.decode(DaemonState.self, "state_full")
        model.apply(.state(full))
        model.setAccountOrder(["claude:main", "codex:work"])
        XCTAssertEqual(model.visibleAccounts.map(\.id), ["claude:main", "codex:work"])
        XCTAssertEqual(model.orderedAccounts.map(\.id), ["claude:main", "codex:work", "codex:hidden"])
        for _ in 0..<100 where model.pendingOrder?.settled != true { await Task.yield() }
        XCTAssertEqual(try sent(transport).last?.params, [.array([.string("claude:main"), .string("codex:work")])])
        model.apply(.state(full))
        XCTAssertEqual(model.visibleAccounts.map(\.id), ["codex:work", "claude:main"])
        await client.stop()
    }

    func testSettingsStoreAppliesChangesAtOnceAndReconciles() async throws {
        let (client, transport) = try await connectedClient()
        let store = SettingsStore(client: client, queue: CommandQueue(client: client))
        await store.reload()
        XCTAssertEqual(store.settings?.display.theme, .system)
        guard case .loaded(let providers) = store.providers else { return XCTFail("expected providers") }
        XCTAssertEqual(providers.first?.id, "codex")
        let write = store.change(.theme(.dark))
        XCTAssertEqual(store.settings?.display.theme, .dark)
        let outcome = await write.value
        XCTAssertNoThrow(try outcome.get())
        XCTAssertEqual(store.settings?.display.theme, .system)
        let requests = try sent(transport)
        XCTAssertEqual(requests.map(\.method), ["GetSettings", "ListProviders", "UpdateSettings", "GetSettings"])
        XCTAssertEqual(requests[2].params, [.string(#"{"display":{"theme":"dark"}}"#)])
        await client.stop()
    }

    func testSettingsStoreKeepsTheErrorOfARejectedPatch() async throws {
        let (client, _) = try await connectedClient(failing: "UpdateSettings")
        let store = SettingsStore(client: client, queue: CommandQueue(client: client))
        await store.reload()
        _ = await store.change(.refreshInterval(120)).value
        XCTAssertEqual(store.lastError, .invalidArguments("rejected"))
        XCTAssertEqual(store.settings?.refreshIntervalSecs, 300)
        await client.stop()
    }
}
