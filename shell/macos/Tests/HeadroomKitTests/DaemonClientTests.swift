import Foundation
import XCTest

@testable import HeadroomKit

final class DaemonClientTests: XCTestCase {
    func testSubscribesThenFetchesState() async throws {
        let transport = FakeTransport(responder: Reply.daemon(state: try Fixture.text("state_full")))
        let client = DaemonClient(transport: transport, sleeper: RecordingSleeper())
        await client.start()
        let events = await collect(client.events) { $0.contains { if case .state = $0 { true } else { false } } }
        await client.stop()
        XCTAssertEqual(events.first, .connecting)
        XCTAssertEqual(events.dropFirst().first, .connected)
        guard case .state(let state) = events.last else { return XCTFail("no state") }
        XCTAssertEqual(state.accounts.count, 3)
        XCTAssertEqual(transport.connections.first?.requests.map(\.method), ["Subscribe", "GetState"])
    }

    func testConcurrentRequestsMatchByIDOutOfOrder() async throws {
        let held = HeldReplies()
        let transport = FakeTransport { request in
            switch request.method {
            case "Subscribe": return [Reply.result(request.id, "null")]
            case "GetState": return [Reply.result(request.id, (try? Fixture.text("state_empty")) ?? "null")]
            default: return held.hold(request)
            }
        }
        let client = DaemonClient(transport: transport, sleeper: RecordingSleeper())
        await client.start()
        _ = await collect(client.events) { $0.contains { if case .state = $0 { true } else { false } } }
        async let first: Void = client.refresh(accountID: "codex:a")
        async let second: Void = client.setAccountLabel(accountID: "codex:b", label: "Work")
        await held.waitForCount(2)
        try held.releaseReversed(on: XCTUnwrap(transport.connections.first))
        try await first
        try await second
        let methods = transport.connections.first?.requests.map(\.method) ?? []
        XCTAssertEqual(Set(methods.suffix(2)), ["Refresh", "SetAccountLabel"])
        await client.stop()
    }

    func testRPCErrorBecomesTypedError() async throws {
        let transport = FakeTransport { request in
            switch request.method {
            case "Refresh": [Reply.error(request.id, code: -32602, message: "unknown account id codex:zz")]
            case "GetState": [Reply.result(request.id, (try? Fixture.text("state_empty")) ?? "null")]
            default: [Reply.result(request.id, "null")]
            }
        }
        let client = DaemonClient(transport: transport, sleeper: RecordingSleeper())
        await client.start()
        _ = await collect(client.events) { $0.contains { if case .state = $0 { true } else { false } } }
        do {
            try await client.refresh(accountID: "codex:zz")
            XCTFail("expected an error")
        } catch {
            XCTAssertEqual(error, .invalidArguments("unknown account id codex:zz"))
        }
        await client.stop()
    }

    func testNotificationsBecomeEvents() async throws {
        let empty = try Fixture.text("state_empty")
        let transport = FakeTransport(responder: Reply.daemon(state: empty))
        let client = DaemonClient(transport: transport, sleeper: RecordingSleeper())
        await client.start()
        _ = await collect(client.events) { $0.contains { if case .state = $0 { true } else { false } } }
        let connection = try XCTUnwrap(transport.connections.first)
        connection.deliver(Reply.notification("StateChanged", #"{"state":\#(try Fixture.text("state_full"))}"#))
        connection.deliver(Reply.notification("OpenRequested", "{}"))
        connection.deliver(
            Reply.notification(
                "Alert", #"{"id":"a","title":"T","body":"B","account_id":null,"urgency":"low"}"#))
        connection.deliver(Reply.notification("FutureThing", "{}"))
        let events = await collect(client.events) { $0.count == 3 }
        guard case .state(let state) = events[0] else { return XCTFail("expected state") }
        XCTAssertEqual(state.headline?.accountID, "claude:main")
        XCTAssertEqual(events[1], .openRequested)
        guard case .alert(let alert) = events[2] else { return XCTFail("expected alert") }
        XCTAssertEqual(alert.urgency, .low)
        XCTAssertNil(alert.accountID)
        await client.stop()
    }

    func testReconnectsWithBackoffAndFailsPendingRequests() async throws {
        let sleeper = RecordingSleeper()
        let transport = FakeTransport(failFirst: 2, responder: Reply.daemon(state: try Fixture.text("state_empty")))
        let client = DaemonClient(transport: transport, sleeper: sleeper)
        await client.start()
        _ = await collect(client.events) { $0.contains { if case .state = $0 { true } else { false } } }
        XCTAssertEqual(sleeper.durations, [.milliseconds(250), .milliseconds(500)])
        transport.connections.first?.drop()
        let events = await collect(client.events) { $0.contains { if case .state = $0 { true } else { false } } }
        XCTAssertEqual(events.first, .disconnected(nil))
        XCTAssertEqual(transport.connections.count, 2)
        await client.stop()
    }

    func testSchemaMismatchIsReportedAndConnectionClosed() async throws {
        let transport = FakeTransport(responder: Reply.daemon(state: #"{"version":2,"future":[]}"#))
        let client = DaemonClient(transport: transport, sleeper: RecordingSleeper())
        await client.start()
        let events = await collect(client.events) { $0.contains(.disconnected(nil)) }
        XCTAssertTrue(events.contains(.failure(.unsupportedSchema(2))))
        await client.stop()
    }

    func testRequestsWithoutConnectionFail() async {
        let client = DaemonClient(transport: FakeTransport(responder: { _ in [] }), sleeper: RecordingSleeper())
        do {
            try await client.rescan()
            XCTFail("expected notConnected")
        } catch {
            XCTAssertEqual(error, .notConnected)
        }
    }

    func testEveryMethodSendsDocumentedParams() async throws {
        let transport = FakeTransport(responder: Reply.daemon(state: try Fixture.text("state_empty")))
        let client = DaemonClient(transport: transport, sleeper: RecordingSleeper())
        await client.start()
        _ = await collect(client.events) { $0.contains { if case .state = $0 { true } else { false } } }
        try await client.refreshNow()
        try await client.rescan()
        try await client.updateSettings(["reduced_motion": .bool(true)])
        try await client.setAccountOrder(["b", "a"])
        try await client.dismissAccount(accountID: "codex:x")
        try await client.restoreAccounts()
        let requests = try XCTUnwrap(transport.connections.first).requests.dropFirst(2)
        XCTAssertEqual(
            requests.map(\.method),
            ["RefreshNow", "Rescan", "UpdateSettings", "SetAccountOrder", "DismissAccount", "RestoreAccounts"])
        XCTAssertEqual(
            requests.map(\.params),
            [
                [], [], [.string(#"{"reduced_motion":true}"#)], [.array([.string("b"), .string("a")])],
                [.string("codex:x")], [.string("")],
            ])
        await client.stop()
    }
}

final class HeldReplies: @unchecked Sendable {
    private let lock = NSLock()
    private var requests: [FakeRequest] = []

    func hold(_ request: FakeRequest) -> [String] {
        lock.withLock { requests.append(request) }
        return []
    }

    func waitForCount(_ count: Int) async {
        while lock.withLock({ requests.count }) < count { await Task.yield() }
    }

    func releaseReversed(on connection: FakeConnection) {
        for request in lock.withLock({ requests }).reversed() {
            connection.deliver(Reply.result(request.id, "null"))
        }
    }
}
