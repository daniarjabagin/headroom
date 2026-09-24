import Foundation

public struct FirstRunFlag {
    static let key = "firstRunCompleted"

    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public var isCompleted: Bool { defaults.bool(forKey: Self.key) }

    public func complete() {
        defaults.set(true, forKey: Self.key)
    }
}
