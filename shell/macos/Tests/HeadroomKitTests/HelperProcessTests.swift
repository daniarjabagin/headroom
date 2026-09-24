import Foundation
import XCTest

@testable import HeadroomKit

final class HelperProcessTests: XCTestCase {
    private let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
        "helper-\(UUID().uuidString)")

    override func setUpWithError() throws {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try FileManager.default.removeItem(at: directory)
    }

    private func script(_ body: String) throws -> URL {
        let url = directory.appendingPathComponent("headroom")
        try Data("#!/bin/sh\n\(body)\n".utf8).write(to: url)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.path)
        return url
    }

    private func collect(_ handle: any HelperProcessHandle) async -> [HelperOutput] {
        var seen: [HelperOutput] = []
        for await output in handle.output { seen.append(output) }
        return seen
    }

    func testStreamsEventsAndForwardsStdin() async throws {
        let helper = try script(
            #"""
            echo "{\"event\":\"started\",\"provider\":\"codex\",\"home\":\"/h\"}"
            echo "args: $*"
            echo "socket: $HEADROOM_SOCKET" >&2
            read -r line
            echo "{\"event\":\"output\",\"line\":\"got $line\"}"
            echo "{\"event\":\"done\",\"account_id\":\"codex:a\",\"label\":null}"
            """#)
        let launcher = HelperLauncher(executable: helper, environment: ["HEADROOM_SOCKET": "/tmp/s.sock"])
        let handle = try launcher.launch(.add(provider: "codex", label: "", apiKeyOnStdin: true))
        handle.write("sk-1")
        handle.closeInput()
        let outputs = await collect(handle)
        XCTAssertEqual(
            outputs,
            [
                .event(.started), .event(.output("args: accounts add codex --api-key-stdin --progress json")),
                .event(.output("socket: /tmp/s.sock")), .event(.output("got sk-1")),
                .event(.done(accountID: "codex:a")),
                .exited(0),
            ])
    }

    func testReportsNonZeroExit() async throws {
        let helper = try script(#"echo "{\"event\":\"error\",\"message\":\"boom\"}"; exit 4"#)
        let handle = try HelperLauncher(executable: helper, environment: [:]).launch(.remove(accountID: "x"))
        let outputs = await collect(handle)
        XCTAssertEqual(outputs, [.event(.error("boom")), .exited(4)])
    }

    func testCancelTerminatesTheProcess() async throws {
        let helper = try script("echo waiting; exec sleep 30")
        let handle = try HelperLauncher(executable: helper, environment: [:]).launch(.remove(accountID: "x"))
        var iterator = handle.output.makeAsyncIterator()
        let first = await iterator.next()
        XCTAssertEqual(first, .event(.output("waiting")))
        handle.cancel()
        handle.write("ignored after close")
        let last = await iterator.next()
        guard case .exited(let status) = last else { return XCTFail("expected exit, got \(String(describing: last))") }
        XCTAssertEqual(status, SIGTERM)
    }

    func testChildStartsWithUnblockedSignalsFromABlockedThread() async throws {
        let helper = try script("echo ready; exec sleep 30")
        let handle = try await Task.detached {
            try SignalMaskFixture.withTerminationBlocked {
                try HelperLauncher(executable: helper, environment: [:]).launch(.remove(accountID: "x"))
            }
        }.value
        var iterator = handle.output.makeAsyncIterator()
        _ = await iterator.next()
        handle.cancel()
        let last = await iterator.next()
        XCTAssertEqual(last, .exited(SIGTERM))
    }

    func testBackToBackLaunchesAllDeliverEndOfOutput() async throws {
        let helper = try script(#"read -r line; echo "{\"event\":\"output\",\"line\":\"$line\"}""#)
        for index in 0..<40 {
            let handle = try HelperLauncher(executable: helper, environment: [:]).launch(.remove(accountID: "x"))
            handle.write("n\(index)")
            handle.closeInput()
            let outputs = await collect(handle)
            XCTAssertEqual(outputs, [.event(.output("n\(index)")), .exited(0)])
        }
    }

    func testMissingHelperFailsToLaunch() {
        XCTAssertThrowsError(try HelperLauncher(executable: nil, environment: [:]).launch(.remove(accountID: "x"))) {
            XCTAssertEqual($0 as? HelperFailure, .missingHelper)
        }
        let absent = directory.appendingPathComponent("absent")
        XCTAssertThrowsError(try HelperLauncher(executable: absent, environment: [:]).launch(.remove(accountID: "x")))
    }
}

private enum SignalMaskFixture {
    static func withTerminationBlocked<Value>(_ body: () throws -> Value) rethrows -> Value {
        var blocked = sigset_t()
        var previous = sigset_t()
        sigemptyset(&blocked)
        sigaddset(&blocked, SIGTERM)
        _ = pthread_sigmask(SIG_BLOCK, &blocked, &previous)
        defer { _ = pthread_sigmask(SIG_SETMASK, &previous, nil) }
        return try body()
    }
}
