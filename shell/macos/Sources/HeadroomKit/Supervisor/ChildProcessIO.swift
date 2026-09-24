import Foundation

extension Process {
    func runWithDefaultSignalMask() throws {
        var previous = sigset_t()
        _ = pthread_sigmask(SIG_SETMASK, nil, &previous)
        var spawnMask = Self.spawnMask(keepingChildSignalFrom: previous)
        _ = pthread_sigmask(SIG_SETMASK, &spawnMask, nil)
        defer { _ = pthread_sigmask(SIG_SETMASK, &previous, nil) }
        try run()
    }

    private static func spawnMask(keepingChildSignalFrom current: sigset_t) -> sigset_t {
        var current = current
        var mask = sigset_t()
        sigemptyset(&mask)
        if sigismember(&current, SIGCHLD) == 1 { sigaddset(&mask, SIGCHLD) }
        return mask
    }
}

enum PipeReader {
    static let chunkSize = 64 * 1024

    static func pump(
        _ handle: FileHandle, chunk: @escaping @Sendable (Data) -> Void, end: @escaping @Sendable () -> Void
    ) {
        Thread {
            var buffer = [UInt8](repeating: 0, count: chunkSize)
            while let data = next(handle.fileDescriptor, into: &buffer) { chunk(data) }
            end()
        }
        .start()
    }

    private static func next(_ descriptor: Int32, into buffer: inout [UInt8]) -> Data? {
        while true {
            let count = buffer.withUnsafeMutableBytes { read(descriptor, $0.baseAddress, $0.count) }
            if count > 0 { return Data(buffer[0..<count]) }
            if count < 0 && errno == EINTR { continue }
            return nil
        }
    }

    static func readToEnd(_ handle: FileHandle) async -> Data {
        await withCheckedContinuation { continuation in
            Thread { continuation.resume(returning: (try? handle.readToEnd()) ?? Data()) }.start()
        }
    }
}
