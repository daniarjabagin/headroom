import Foundation
import Observation

public enum WelcomeExit: Sendable, Hashable {
    case openHeadroom, settings, dismissed
}

public enum WelcomeStep: Sendable, Hashable {
    case found, menuBar
}

@MainActor
@Observable
public final class WelcomeFlow {
    public var openAtLogin = true {
        didSet { failure = nil }
    }
    public private(set) var failure: String?
    public private(set) var isFinished = false
    public private(set) var step: WelcomeStep

    @ObservationIgnored public var onFinish: (@MainActor (WelcomeExit) -> Void)?
    @ObservationIgnored private let flag: FirstRunFlag
    @ObservationIgnored private let steps: [WelcomeStep]
    @ObservationIgnored private let completeOnboarding: @MainActor () -> Void
    @ObservationIgnored private let enableLoginItem: @MainActor () -> String?

    public init(
        flag: FirstRunFlag, steps: [WelcomeStep] = [.menuBar],
        completeOnboarding: @escaping @MainActor () -> Void = {},
        enableLoginItem: @escaping @MainActor () -> String?
    ) {
        self.flag = flag
        self.steps = steps.isEmpty ? [.menuBar] : steps
        self.step = steps.first ?? .menuBar
        self.completeOnboarding = completeOnboarding
        self.enableLoginItem = enableLoginItem
    }

    public func start() {
        guard !isFinished, step == .found else { return }
        completeOnboarding()
        guard let next = steps.drop(while: { $0 != .found }).dropFirst().first else {
            finish(.openHeadroom)
            return
        }
        step = next
    }

    public func chooseLater() {
        guard !isFinished, step == .found else { return }
        finish(.dismissed)
    }

    public func choose(_ exit: WelcomeExit) {
        guard !isFinished else { return }
        guard step == .menuBar else {
            if exit == .dismissed { chooseLater() }
            return
        }
        if exit != .dismissed, openAtLogin {
            failure = enableLoginItem()
            guard failure == nil else { return }
        }
        flag.complete()
        finish(exit)
    }

    private func finish(_ exit: WelcomeExit) {
        isFinished = true
        onFinish?(exit)
    }
}

public enum WelcomeDecision: Sendable, Hashable {
    case wait
    case skip
    case show([WelcomeStep])
}

public enum WelcomePlan {
    public static func decide(firstRunCompleted: Bool, settings: Settings?, timedOut: Bool) -> WelcomeDecision {
        guard settings != nil || (!firstRunCompleted && timedOut) else { return .wait }
        let steps = steps(firstRunCompleted: firstRunCompleted, settings: settings)
        return steps.isEmpty ? .skip : .show(steps)
    }

    public static func steps(firstRunCompleted: Bool, settings: Settings?) -> [WelcomeStep] {
        var steps: [WelcomeStep] = []
        if let settings, offersOnboarding(settings) { steps.append(.found) }
        if !firstRunCompleted { steps.append(.menuBar) }
        return steps
    }

    public static func offersOnboarding(_ settings: Settings) -> Bool {
        settings.features.release06 && !settings.onboarding.completed
    }
}
