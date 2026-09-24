import Foundation

public struct UnixSocketTransport: LineTransport {
    public let path: String

    public init(path: String) {
        self.path = path
    }

    public func open() async throws -> any LineConnection {
        let descriptor = try POSIXSocket.connect(path: path)
        return UnixSocketConnection(descriptor: descriptor)
    }
}

final class UnixSocketConnection: LineConnection, @unchecked Sendable {
    let lines: AsyncThrowingStream<Data, any Error>

    private let descriptor: Int32
    private let lock = NSLock()
    private var isClosed = false

    init(descriptor: Int32) {
        self.descriptor = descriptor
        let (stream, continuation) = AsyncThrowingStream.makeStream(of: Data.self)
        lines = stream
        Thread { [self] in readLoop(continuation) }.start()
    }

    func send(_ line: Data) async throws {
        try sendLocked(line)
    }

    private func sendLocked(_ line: Data) throws(DaemonError) {
        lock.lock()
        defer { lock.unlock() }
        guard !isClosed else { throw .disconnected }
        try POSIXSocket.send(descriptor, line)
    }

    func close() async {
        lock.withLock {
            if !isClosed { POSIXSocket.shutdownBoth(descriptor) }
        }
    }

    private func readLoop(_ continuation: AsyncThrowingStream<Data, any Error>.Continuation) {
        let failure = pump(into: continuation)
        lock.withLock {
            isClosed = true
            POSIXSocket.closeDescriptor(descriptor)
        }
        continuation.finish(throwing: failure)
    }

    private func pump(into continuation: AsyncThrowingStream<Data, any Error>.Continuation) -> DaemonError? {
        var buffer = [UInt8](repeating: 0, count: 64 * 1024)
        var framer = LineBuffer()
        while true {
            let count = POSIXSocket.receive(descriptor, into: &buffer)
            if count == 0 { return nil }
            if count < 0 {
                if errno == EINTR { continue }
                return POSIXSocket.failure("recv")
            }
            do {
                for line in try framer.append(Data(buffer[0..<count])) { continuation.yield(line) }
            } catch {
                return error
            }
        }
    }
}
