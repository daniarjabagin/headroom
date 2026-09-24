import Foundation

public enum RefreshMode: Sendable, Hashable {
    case idle, busy, failed
}

public struct RefreshTracker: Sendable, Hashable {
    public static let minimumSpin: TimeInterval = 0.8
    public static let errorDuration: TimeInterval = 1

    private var inFlight = false
    private var holdUntil = Date.distantPast
    private var errorUntil = Date.distantPast
    private var daemonBusy = false

    public init() {}

    public mutating func press(at now: Date) -> Bool {
        guard !inFlight, now >= holdUntil else { return false }
        inFlight = true
        holdUntil = now.addingTimeInterval(Self.minimumSpin)
        errorUntil = .distantPast
        return true
    }

    public mutating func settle(succeeded: Bool, at now: Date) {
        inFlight = false
        guard !succeeded else { return }
        holdUntil = .distantPast
        errorUntil = now.addingTimeInterval(Self.errorDuration)
    }

    public mutating func setDaemonBusy(_ busy: Bool) {
        daemonBusy = busy
    }

    public func mode(at now: Date) -> RefreshMode {
        if now < errorUntil { return .failed }
        if inFlight || now < holdUntil || daemonBusy { return .busy }
        return .idle
    }

    public func nextChange(after now: Date) -> Date? {
        [holdUntil, errorUntil].filter { $0 > now }.min()
    }
}
