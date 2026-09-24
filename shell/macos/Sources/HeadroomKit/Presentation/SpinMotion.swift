import Foundation

public struct SpinMotion: Sendable, Hashable {
    public static let turn: TimeInterval = 1.4
    static let fullTurn = 360.0
    static let quarterTurn = 90.0
    static var easeDuration: TimeInterval { 2 * quarterTurn / fullTurn * turn }
    static var speed: Double { fullTurn / turn }

    private enum Phase: Sendable, Hashable {
        case resting
        case spinning(since: Date, from: Double)
        case stopping(since: Date, from: Double, cruise: Double)
    }

    private var phase = Phase.resting

    public init() {}

    public var isSpinning: Bool {
        if case .spinning = phase { return true }
        return false
    }

    public mutating func start(at now: Date) {
        guard !isSpinning else { return }
        phase = .spinning(since: now, from: Self.resting(angle(at: now)))
    }

    public mutating func stop(at now: Date) {
        guard isSpinning else { return }
        let from = Self.resting(angle(at: now))
        guard from > 0 else {
            phase = .resting
            return
        }
        let left = Self.fullTurn - from
        let distance = left < Self.quarterTurn ? left + Self.fullTurn : left
        phase = .stopping(since: now, from: from, cruise: distance - Self.quarterTurn)
    }

    public func restsAt() -> Date? {
        guard case .stopping(let since, _, let cruise) = phase else { return nil }
        return since.addingTimeInterval(cruise / Self.speed + Self.easeDuration)
    }

    public func isAnimating(at now: Date) -> Bool {
        switch phase {
        case .resting: false
        case .spinning: true
        case .stopping: (restsAt() ?? now) > now
        }
    }

    public func angle(at now: Date) -> Double {
        switch phase {
        case .resting:
            return 0
        case .spinning(let since, let from):
            return from + spinDistance(max(0, now.timeIntervalSince(since)))
        case .stopping(let since, let from, let cruise):
            return from + stopDistance(max(0, now.timeIntervalSince(since)), cruise: cruise)
        }
    }

    static func resting(_ angle: Double) -> Double {
        let remainder = angle.truncatingRemainder(dividingBy: fullTurn)
        return remainder < 0 ? remainder + fullTurn : remainder
    }

    private func spinDistance(_ elapsed: TimeInterval) -> Double {
        let ease = Self.easeDuration
        guard elapsed >= ease else {
            let progress = elapsed / ease
            return Self.quarterTurn * progress * progress
        }
        return Self.quarterTurn + (elapsed - ease) * Self.speed
    }

    private func stopDistance(_ elapsed: TimeInterval, cruise: Double) -> Double {
        let cruiseTime = cruise / Self.speed
        guard elapsed >= cruiseTime else { return elapsed * Self.speed }
        let progress = min(1, (elapsed - cruiseTime) / Self.easeDuration)
        return cruise + Self.quarterTurn * (1 - (1 - progress) * (1 - progress))
    }
}

public enum ShakeMotion {
    public static let step: TimeInterval = 0.066
    static let offsets: [Double] = [3, -3, 2, -2, 1, 0]

    public static var duration: TimeInterval { step * Double(offsets.count) }

    public static func offset(elapsed: TimeInterval) -> Double {
        guard elapsed > 0 else { return 0 }
        let index = Int(elapsed / step)
        guard index < offsets.count else { return 0 }
        let from = index == 0 ? 0 : offsets[index - 1]
        let progress = (elapsed - Double(index) * step) / step
        let eased = (1 - cos(progress * .pi)) / 2
        return from + (offsets[index] - from) * eased
    }
}
