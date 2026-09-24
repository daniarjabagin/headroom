import Foundation
import Observation

public struct SignInProgress: Sendable, Hashable {
    public static let logLimit = 500

    public var url: String?
    public var deviceCode: String?
    public var log: [String] = []

    mutating func record(_ line: String) {
        log.append(line)
        if log.count > Self.logLimit { log.removeFirst(log.count - Self.logLimit) }
        if let code = DeviceCode.find(in: line) { deviceCode = code }
    }
}

public enum AddAccountPhase: Sendable, Hashable {
    case form
    case running(SignInProgress)
    case done(accountID: String?)
    case failed(HelperFailure)
}

@MainActor
@Observable
public final class AddAccountSession {
    public let provider: ProviderInfo
    public private(set) var phase: AddAccountPhase = .form

    private let launcher: any HelperLaunching
    private var handle: (any HelperProcessHandle)?
    private var pump: Task<Void, Never>?

    public init(provider: ProviderInfo, launcher: any HelperLaunching) {
        self.provider = provider
        self.launcher = launcher
    }

    public var isRunning: Bool {
        if case .running = phase { return true }
        return false
    }

    public func start(label: String, apiKey: String? = nil) {
        guard !isRunning else { return }
        let command = AccountCommand.add(provider: provider.id, label: label, apiKeyOnStdin: apiKey != nil)
        do {
            let handle = try launcher.launch(command)
            self.handle = handle
            phase = .running(SignInProgress())
            if let apiKey {
                handle.write(apiKey)
                handle.closeInput()
            }
            pump = Task { [weak self] in
                for await output in handle.output { self?.receive(output) }
            }
        } catch {
            phase = .failed(error)
        }
    }

    public func sendCode(_ code: String) {
        let trimmed = code.trimmingCharacters(in: .whitespacesAndNewlines)
        guard isRunning, !trimmed.isEmpty else { return }
        handle?.write(trimmed)
    }

    public func cancel() {
        stop()
        phase = .form
    }

    func receive(_ output: HelperOutput) {
        guard case .running(var progress) = phase else { return }
        switch output {
        case .event(.started): return
        case .event(.url(let url)):
            if progress.url == nil { progress.url = url }
            progress.record(url)
            phase = .running(progress)
        case .event(.output(let line)):
            progress.record(line)
            phase = .running(progress)
        case .event(.done(let accountID)): finish(.done(accountID: accountID))
        case .event(.error(let message)): finish(.failed(.message(message)))
        case .exited(let status): finish(status == 0 ? .done(accountID: nil) : .failed(.exitStatus(status)))
        }
    }

    private func finish(_ outcome: AddAccountPhase) {
        phase = outcome
        if case .failed = outcome { stop() }
    }

    private func stop() {
        pump?.cancel()
        pump = nil
        handle?.cancel()
        handle = nil
    }
}

public enum AccountRemoval {
    public static func remove(accountID: String, launcher: any HelperLaunching) async -> Result<Void, HelperFailure> {
        let handle: any HelperProcessHandle
        do {
            handle = try launcher.launch(.remove(accountID: accountID))
        } catch {
            return .failure(error)
        }
        var failure: HelperFailure?
        for await output in handle.output {
            switch output {
            case .event(.error(let message)): failure = failure ?? .message(message)
            case .exited(let status) where status != 0: failure = failure ?? .exitStatus(status)
            default: continue
            }
        }
        return failure.map(Result.failure) ?? .success(())
    }
}
