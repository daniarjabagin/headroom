import Foundation

public struct LaunchSpec: Sendable, Hashable {
    public let executable: URL
    public let arguments: [String]
    public let environment: [String: String]
    public let logFile: URL?

    public init(executable: URL, arguments: [String], environment: [String: String], logFile: URL?) {
        self.executable = executable
        self.arguments = arguments
        self.environment = environment
        self.logFile = logFile
    }

    public static func daemon(
        helper: URL, socketPath: String, environment: [String: String], logFile: URL?
    ) -> LaunchSpec {
        LaunchSpec(
            executable: helper, arguments: ["daemon", "--socket", socketPath], environment: environment,
            logFile: logFile)
    }
}

public protocol SupervisedProcess: Sendable {
    func waitForExit() async -> Int32
    func terminate()
    func kill()
}

public protocol ProcessLauncher: Sendable {
    func launch(_ spec: LaunchSpec) throws -> any SupervisedProcess
}

public struct FoundationProcessLauncher: ProcessLauncher {
    public init() {}

    public func launch(_ spec: LaunchSpec) throws -> any SupervisedProcess {
        let process = Process()
        process.executableURL = spec.executable
        process.arguments = spec.arguments
        process.environment = spec.environment
        process.standardInput = FileHandle.nullDevice
        let output = try spec.logFile.map(LogFile.openForAppending) ?? FileHandle.nullDevice
        process.standardOutput = output
        process.standardError = output
        let running = FoundationProcess(process: process)
        try process.run()
        return running
    }
}

final class FoundationProcess: SupervisedProcess, @unchecked Sendable {
    private let process: Process
    private let exit = ExitSignal()

    init(process: Process) {
        self.process = process
        process.terminationHandler = { [exit] finished in exit.finish(finished.terminationStatus) }
    }

    func waitForExit() async -> Int32 {
        await exit.wait()
    }

    func terminate() {
        if process.isRunning { process.terminate() }
    }

    func kill() {
        guard process.isRunning else { return }
        _ = Foundation.kill(process.processIdentifier, SIGKILL)
    }
}

final class ExitSignal: @unchecked Sendable {
    private let lock = NSLock()
    private var status: Int32?
    private var waiters: [CheckedContinuation<Int32, Never>] = []

    func finish(_ code: Int32) {
        let pending = lock.withLock {
            status = code
            defer { waiters.removeAll() }
            return waiters
        }
        for waiter in pending { waiter.resume(returning: code) }
    }

    func wait() async -> Int32 {
        await withCheckedContinuation { continuation in
            let finished = lock.withLock {
                if status == nil { waiters.append(continuation) }
                return status
            }
            if let finished { continuation.resume(returning: finished) }
        }
    }
}
