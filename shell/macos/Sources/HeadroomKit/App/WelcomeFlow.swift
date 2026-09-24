import Foundation
import Observation

public enum WelcomeExit: Sendable, Hashable {
    case openHeadroom, settings, dismissed
}

@MainActor
@Observable
public final class WelcomeFlow {
    public var openAtLogin = true {
        didSet { failure = nil }
    }
    public private(set) var failure: String?
    public private(set) var isFinished = false

    @ObservationIgnored public var onFinish: (@MainActor (WelcomeExit) -> Void)?
    @ObservationIgnored private let flag: FirstRunFlag
    @ObservationIgnored private let enableLoginItem: @MainActor () -> String?

    public init(flag: FirstRunFlag, enableLoginItem: @escaping @MainActor () -> String?) {
        self.flag = flag
        self.enableLoginItem = enableLoginItem
    }

    public func choose(_ exit: WelcomeExit) {
        guard !isFinished else { return }
        if exit != .dismissed, openAtLogin {
            failure = enableLoginItem()
            guard failure == nil else { return }
        }
        isFinished = true
        flag.complete()
        onFinish?(exit)
    }
}
