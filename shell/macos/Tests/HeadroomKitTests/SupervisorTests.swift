import Foundation
import XCTest

@testable import HeadroomKit

final class BackoffTests: XCTestCase {
    func testDoublesUpToMaximum() {
        let backoff = Backoff(initial: .seconds(1), maximum: .seconds(60))
        XCTAssertEqual((0...7).map(backoff.delay), [1, 2, 4, 8, 16, 32, 60, 60].map { Duration.seconds($0) })
        XCTAssertEqual(backoff.delay(attempt: 10_000), .seconds(60))
        XCTAssertEqual(backoff.delay(attempt: -3), .seconds(1))
    }

    func testRestartPolicyResetsAfterStableUptime() {
        let policy = RestartPolicy.standard
        let quick = policy.decide(consecutiveFailures: 3, uptime: .seconds(2))
        XCTAssertEqual(quick, RestartDecision(delay: .seconds(8), consecutiveFailures: 4))
        let stable = policy.decide(consecutiveFailures: 3, uptime: .seconds(45))
        XCTAssertEqual(stable, RestartDecision(delay: .seconds(1), consecutiveFailures: 1))
        XCTAssertEqual(policy.decide(consecutiveFailures: 0, uptime: .zero).delay, .seconds(1))
    }
}

final class DaemonSupervisorTests: XCTestCase {
    private let spec = LaunchSpec.daemon(
        helper: URL(fileURLWithPath: "/Applications/Headroom.app/Contents/Helpers/headroom"),
        socketPath: "/tmp/h.sock", environment: ["PATH": "/usr/bin"], logFile: nil)

    func testLaunchSpecPassesSocket() {
        XCTAssertEqual(spec.arguments, ["daemon", "--socket", "/tmp/h.sock", "--no-update-check"])
    }

    func testRestartsCrashedDaemonWithGrowingDelay() async {
        let launcher = FakeLauncher(exitStatuses: [1, 1, 1])
        let sleeper = RecordingSleeper()
        let supervisor = DaemonSupervisor(
            spec: spec, launcher: launcher, clock: MonotonicClock { .zero }, sleeper: sleeper)
        await supervisor.start()
        var exits: [SupervisorEvent] = []
        for await event in supervisor.events {
            if case .exited = event { exits.append(event) }
            if exits.count == 3 { break }
        }
        await supervisor.stop()
        XCTAssertEqual(
            exits,
            [
                .exited(status: 1, restartIn: .seconds(1)), .exited(status: 1, restartIn: .seconds(2)),
                .exited(status: 1, restartIn: .seconds(4)),
            ])
        XCTAssertEqual(launcher.specs.first, spec)
    }

    func testLaunchFailureIsReportedAndRetried() async {
        let launcher = FakeLauncher(exitStatuses: [], failLaunch: true)
        let supervisor = DaemonSupervisor(
            spec: spec, launcher: launcher, clock: MonotonicClock { .zero }, sleeper: RecordingSleeper())
        await supervisor.start()
        var iterator = supervisor.events.makeAsyncIterator()
        let event = await iterator.next()
        await supervisor.stop()
        XCTAssertEqual(event, .launchFailed(message: "missing helper", retryIn: .seconds(1)))
    }

    func testStopTerminatesRunningDaemon() async {
        let launcher = FakeLauncher(exitStatuses: [])
        let supervisor = DaemonSupervisor(
            spec: spec, launcher: launcher, clock: MonotonicClock { .zero }, sleeper: RecordingSleeper())
        await supervisor.start()
        var iterator = supervisor.events.makeAsyncIterator()
        let first = await iterator.next()
        XCTAssertEqual(first, .started)
        await supervisor.stop()
        XCTAssertEqual(launcher.processes.first?.terminated, true)
        XCTAssertEqual(launcher.specs.count, 1)
    }
}

private struct LaunchFailure: Error, CustomStringConvertible {
    var description: String { "missing helper" }
}

private final class FakeLauncher: ProcessLauncher, @unchecked Sendable {
    private let lock = NSLock()
    private var statuses: [Int32]
    private let failLaunch: Bool
    private var launched: [LaunchSpec] = []
    private var running: [FakeProcess] = []

    init(exitStatuses: [Int32], failLaunch: Bool = false) {
        statuses = exitStatuses
        self.failLaunch = failLaunch
    }

    var specs: [LaunchSpec] { lock.withLock { launched } }
    var processes: [FakeProcess] { lock.withLock { running } }

    func launch(_ spec: LaunchSpec) throws -> any SupervisedProcess {
        if failLaunch { throw LaunchFailure() }
        return lock.withLock {
            launched.append(spec)
            let process = FakeProcess(exitImmediately: statuses.isEmpty ? nil : statuses.removeFirst())
            running.append(process)
            return process
        }
    }
}

private final class FakeProcess: SupervisedProcess, @unchecked Sendable {
    private let exit = ExitSignal()
    private let lock = NSLock()
    private var wasTerminated = false

    init(exitImmediately status: Int32?) {
        if let status { exit.finish(status) }
    }

    var terminated: Bool { lock.withLock { wasTerminated } }

    func waitForExit() async -> Int32 {
        await exit.wait()
    }

    func terminate() {
        lock.withLock { wasTerminated = true }
        exit.finish(15)
    }

    func kill() {
        exit.finish(9)
    }
}
