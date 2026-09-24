public struct Backoff: Sendable, Hashable {
    public let initial: Duration
    public let maximum: Duration

    public init(initial: Duration, maximum: Duration) {
        self.initial = initial
        self.maximum = maximum
    }

    public static let reconnect = Backoff(initial: .milliseconds(250), maximum: .seconds(10))
    public static let restart = Backoff(initial: .seconds(1), maximum: .seconds(60))

    public func delay(attempt: Int) -> Duration {
        var delay = initial
        for _ in 0..<max(0, attempt) {
            delay *= 2
            if delay >= maximum { return maximum }
        }
        return min(delay, maximum)
    }
}

public struct RestartPolicy: Sendable, Hashable {
    public let backoff: Backoff
    public let stableUptime: Duration

    public init(backoff: Backoff, stableUptime: Duration) {
        self.backoff = backoff
        self.stableUptime = stableUptime
    }

    public static let standard = RestartPolicy(backoff: .restart, stableUptime: .seconds(30))

    public func decide(consecutiveFailures: Int, uptime: Duration) -> RestartDecision {
        let failures = uptime >= stableUptime ? 0 : consecutiveFailures
        return RestartDecision(delay: backoff.delay(attempt: failures), consecutiveFailures: failures + 1)
    }
}

public struct RestartDecision: Sendable, Hashable {
    public let delay: Duration
    public let consecutiveFailures: Int
}

public protocol Sleeper: Sendable {
    func sleep(for duration: Duration) async throws
}

public struct TaskSleeper: Sleeper {
    public init() {}

    public func sleep(for duration: Duration) async throws {
        try await Task.sleep(for: duration)
    }
}

public struct MonotonicClock: Sendable {
    public let now: @Sendable () -> Duration

    public init(now: @escaping @Sendable () -> Duration) {
        self.now = now
    }

    public static func system() -> MonotonicClock {
        let origin = ContinuousClock.now
        return MonotonicClock { ContinuousClock.now - origin }
    }
}
