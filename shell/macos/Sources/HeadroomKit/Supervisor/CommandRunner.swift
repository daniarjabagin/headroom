import Foundation

public struct CommandOutput: Sendable, Hashable {
    public let status: Int32
    public let stdout: String
}

public enum CommandError: Error, Sendable, Hashable {
    case launchFailed(String)
    case timedOut
}

public protocol CommandRunning: Sendable {
    func run(
        _ executable: URL, arguments: [String], environment: [String: String]?, timeout: Duration
    ) async throws(CommandError) -> CommandOutput

    func output(
        of executable: URL, arguments: [String], until terminator: String, timeout: Duration
    ) async throws(CommandError) -> String
}

public struct CommandRunner: CommandRunning {
    public init() {}

    public func run(
        _ executable: URL, arguments: [String], environment: [String: String]?, timeout: Duration
    ) async throws(CommandError) -> CommandOutput {
        let launched = try Launched(executable, arguments: arguments, environment: environment)
        return try await launched.finish(within: timeout) { stop in
            async let data = OutputDrain.collect(launched.reader, stop: stop, terminator: nil)
            let status = await launched.process.waitForExit()
            stop.processExited()
            return CommandOutput(status: status, stdout: String(decoding: await data, as: UTF8.self))
        }
    }

    public func output(
        of executable: URL, arguments: [String], until terminator: String, timeout: Duration
    ) async throws(CommandError) -> String {
        let launched = try Launched(executable, arguments: arguments, environment: nil)
        return try await launched.finish(within: timeout) { stop in
            Task.detached {
                _ = await launched.process.waitForExit()
                stop.processExited()
            }
            let data = await OutputDrain.collect(launched.reader, stop: stop, terminator: Data(terminator.utf8))
            return String(decoding: data, as: UTF8.self)
        }
    }
}

private struct Launched: Sendable {
    let process: FoundationProcess
    let reader: FileHandle

    init(_ executable: URL, arguments: [String], environment: [String: String]?) throws(CommandError) {
        let process = Process()
        process.executableURL = executable
        process.arguments = arguments
        if let environment { process.environment = environment }
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = FileHandle.nullDevice
        process.standardInput = FileHandle.nullDevice
        self.process = FoundationProcess(process: process)
        reader = pipe.fileHandleForReading
        do {
            try process.runWithDefaultSignalMask()
        } catch {
            throw .launchFailed(String(describing: error))
        }
    }

    func finish<Value: Sendable>(
        within timeout: Duration, _ body: @escaping @Sendable (DrainStop) async -> Value
    ) async throws(CommandError) -> Value {
        let stop = DrainStop()
        let result: Value? = await withCheckedContinuation { continuation in
            let once = ResumeOnce<Value?>(continuation)
            let timer = Task.detached { [process] in
                guard (try? await Task.sleep(for: timeout)) != nil else { return }
                stop.abandon()
                process.kill()
                once.resume(nil)
            }
            Task.detached {
                once.resume(await body(stop))
                timer.cancel()
            }
        }
        guard let result else { throw .timedOut }
        return result
    }
}

private final class ResumeOnce<Value: Sendable>: @unchecked Sendable {
    private let lock = NSLock()
    private var continuation: CheckedContinuation<Value, Never>?

    init(_ continuation: CheckedContinuation<Value, Never>) {
        self.continuation = continuation
    }

    func resume(_ value: Value) {
        let pending = lock.withLock {
            defer { continuation = nil }
            return continuation
        }
        pending?.resume(returning: value)
    }
}
