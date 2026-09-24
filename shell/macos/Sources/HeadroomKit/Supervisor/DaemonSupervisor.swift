import Foundation

public enum SupervisorEvent: Sendable, Equatable {
    case started
    case exited(status: Int32, restartIn: Duration)
    case launchFailed(message: String, retryIn: Duration)
}

public actor DaemonSupervisor {
    public static let terminationGrace: Duration = .seconds(3)

    public nonisolated let events: AsyncStream<SupervisorEvent>

    private let emitter: AsyncStream<SupervisorEvent>.Continuation
    private let spec: LaunchSpec
    private let launcher: any ProcessLauncher
    private let policy: RestartPolicy
    private let clock: MonotonicClock
    private let sleeper: any Sleeper
    private var current: (any SupervisedProcess)?
    private var loop: Task<Void, Never>?

    public init(
        spec: LaunchSpec, launcher: any ProcessLauncher = FoundationProcessLauncher(),
        policy: RestartPolicy = .standard, clock: MonotonicClock = .system(),
        sleeper: any Sleeper = TaskSleeper()
    ) {
        self.spec = spec
        self.launcher = launcher
        self.policy = policy
        self.clock = clock
        self.sleeper = sleeper
        (events, emitter) = AsyncStream.makeStream(of: SupervisorEvent.self, bufferingPolicy: .bufferingNewest(32))
    }

    public func start() {
        guard loop == nil else { return }
        loop = Task { await self.run() }
    }

    public func stop() async {
        let running = loop
        loop = nil
        running?.cancel()
        current?.terminate()
        await waitForShutdown(running)
    }

    private func waitForShutdown(_ running: Task<Void, Never>?) async {
        guard let running else { return }
        let process = current
        let sleeper = sleeper
        let killer = Task {
            try await sleeper.sleep(for: Self.terminationGrace)
            process?.kill()
        }
        await running.value
        killer.cancel()
    }

    private func run() async {
        var failures = 0
        while !Task.isCancelled {
            let startedAt = clock.now()
            let outcome = await runOnce()
            guard !Task.isCancelled else { break }
            let decision = policy.decide(consecutiveFailures: failures, uptime: clock.now() - startedAt)
            failures = decision.consecutiveFailures
            emitter.yield(outcome.event(restartIn: decision.delay))
            try? await sleeper.sleep(for: decision.delay)
        }
    }

    private func runOnce() async -> RunOutcome {
        do {
            let process = try launcher.launch(spec)
            current = process
            emitter.yield(.started)
            let status = await process.waitForExit()
            current = nil
            return .exited(status)
        } catch {
            return .launchFailed(String(describing: error))
        }
    }
}

private enum RunOutcome {
    case exited(Int32)
    case launchFailed(String)

    func event(restartIn delay: Duration) -> SupervisorEvent {
        switch self {
        case .exited(let status): .exited(status: status, restartIn: delay)
        case .launchFailed(let message): .launchFailed(message: message, retryIn: delay)
        }
    }
}
