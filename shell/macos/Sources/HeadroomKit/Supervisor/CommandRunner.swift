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
}

public struct CommandRunner: CommandRunning {
    public init() {}

    public func run(
        _ executable: URL, arguments: [String], environment: [String: String]?, timeout: Duration
    ) async throws(CommandError) -> CommandOutput {
        let process = Process()
        process.executableURL = executable
        process.arguments = arguments
        if let environment { process.environment = environment }
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = FileHandle.nullDevice
        process.standardInput = FileHandle.nullDevice
        let running = FoundationProcess(process: process)
        do {
            try process.runWithDefaultSignalMask()
        } catch {
            throw .launchFailed(String(describing: error))
        }
        return try await collect(running, reader: pipe.fileHandleForReading, timeout: timeout)
    }

    private func collect(
        _ running: FoundationProcess, reader: FileHandle, timeout: Duration
    ) async throws(CommandError) -> CommandOutput {
        let result: CommandOutput? = await withCheckedContinuation { continuation in
            let once = ResumeOnce(continuation)
            Task.detached { once.resume(await Self.drain(running, reader: reader)) }
            Task.detached {
                try? await Task.sleep(for: timeout)
                running.kill()
                once.resume(nil)
            }
        }
        guard let result else { throw .timedOut }
        return result
    }

    private static func drain(_ running: FoundationProcess, reader: FileHandle) async -> CommandOutput {
        let data = await PipeReader.readToEnd(reader)
        let status = await running.waitForExit()
        return CommandOutput(status: status, stdout: String(decoding: data, as: UTF8.self))
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
