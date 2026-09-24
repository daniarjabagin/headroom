import Foundation

public enum HelperOutput: Sendable, Hashable {
    case event(AccountProgressEvent)
    case exited(Int32)
}

public protocol HelperProcessHandle: Sendable {
    var output: AsyncStream<HelperOutput> { get }
    func write(_ line: String)
    func closeInput()
    func cancel()
}

public protocol HelperLaunching: Sendable {
    func launch(_ command: AccountCommand) throws(HelperFailure) -> any HelperProcessHandle
}

public struct HelperLauncher: HelperLaunching {
    public static let terminationGrace: Duration = .seconds(3)

    public let executable: URL?
    public let environment: [String: String]

    public init(executable: URL?, environment: [String: String]) {
        self.executable = executable
        self.environment = environment
    }

    public func launch(_ command: AccountCommand) throws(HelperFailure) -> any HelperProcessHandle {
        guard let executable else { throw .missingHelper }
        let process = Process()
        process.executableURL = executable
        process.arguments = command.arguments
        process.environment = environment
        let input = Pipe()
        let output = Pipe()
        process.standardInput = input
        process.standardOutput = output
        process.standardError = output
        let running = RunningHelper(process: process, input: input.fileHandleForWriting)
        do {
            try process.runWithDefaultSignalMask()
        } catch {
            throw .launchFailed(String(describing: error))
        }
        running.read(from: output.fileHandleForReading)
        return running
    }
}

final class RunningHelper: HelperProcessHandle, @unchecked Sendable {
    let output: AsyncStream<HelperOutput>

    private let emitter: AsyncStream<HelperOutput>.Continuation
    private let process: FoundationProcess
    private let input: FileHandle
    private let lock = NSLock()
    private var buffer = LineBuffer()
    private var inputClosed = false

    init(process: Process, input: FileHandle) {
        self.process = FoundationProcess(process: process)
        self.input = input
        (output, emitter) = AsyncStream.makeStream(of: HelperOutput.self)
    }

    func read(from reader: FileHandle) {
        PipeReader.pump(reader, chunk: { self.consume($0) }, end: { self.finish() })
    }

    func write(_ line: String) {
        lock.withLock {
            guard !inputClosed else { return }
            try? input.write(contentsOf: Data((line + "\n").utf8))
        }
    }

    func closeInput() {
        lock.withLock {
            guard !inputClosed else { return }
            inputClosed = true
            try? input.close()
        }
    }

    func cancel() {
        closeInput()
        process.terminate()
        let process = process
        Task.detached {
            try? await Task.sleep(for: HelperLauncher.terminationGrace)
            process.kill()
        }
    }

    private func consume(_ chunk: Data) {
        let lines: [Data]
        do {
            lines = try lock.withLock { try buffer.append(chunk) }
        } catch {
            emitter.yield(.event(.error(String(describing: error))))
            cancel()
            return
        }
        for line in lines {
            guard let event = AccountProgressEvent.parse(String(decoding: line, as: UTF8.self)) else { continue }
            emitter.yield(.event(event))
        }
    }

    private func finish() {
        closeInput()
        let process = process
        let emitter = emitter
        Task.detached {
            emitter.yield(.exited(await process.waitForExit()))
            emitter.finish()
        }
    }
}
