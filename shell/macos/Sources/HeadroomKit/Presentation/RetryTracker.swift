import Foundation

public struct RetryTracker: Sendable, Hashable {
    public static let timeout: TimeInterval = 10

    private var pressedAt: [String: Date] = [:]
    private var refreshing: Set<String> = []

    public init() {}

    public mutating func press(_ accountID: String, at now: Date) -> Bool {
        guard !isPending(accountID, at: now) else { return false }
        pressedAt[accountID] = now
        return true
    }

    public mutating func settle(_ accountID: String, succeeded: Bool) {
        guard !succeeded || refreshing.contains(accountID) else { return }
        pressedAt[accountID] = nil
    }

    public mutating func observe(refreshing ids: Set<String>) {
        refreshing = ids
        pressedAt = pressedAt.filter { !ids.contains($0.key) }
    }

    public func isPending(_ accountID: String, at now: Date) -> Bool {
        guard let pressed = pressedAt[accountID] else { return false }
        return now.timeIntervalSince(pressed) < Self.timeout
    }

    public func nextExpiry(after now: Date) -> Date? {
        pressedAt.values.map { $0.addingTimeInterval(Self.timeout) }.filter { $0 > now }.min()
    }
}
