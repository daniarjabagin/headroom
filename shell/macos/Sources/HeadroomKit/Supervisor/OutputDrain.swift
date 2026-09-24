import Foundation

final class DrainStop: @unchecked Sendable {
    enum State {
        case running
        case processExited
        case abandoned
    }

    private let lock = NSLock()
    private var current = State.running

    var state: State { lock.withLock { current } }

    func processExited() {
        lock.withLock { if current == .running { current = .processExited } }
    }

    func abandon() {
        lock.withLock { current = .abandoned }
    }
}

enum OutputDrain {
    static let pollInterval: Int32 = 50

    static func collect(_ handle: FileHandle, stop: DrainStop, terminator: Data?) async -> Data {
        await withCheckedContinuation { continuation in
            Thread { continuation.resume(returning: drain(handle, stop: stop, terminator: terminator)) }.start()
        }
    }

    private static func drain(_ handle: FileHandle, stop: DrainStop, terminator: Data?) -> Data {
        var collected = Data()
        var buffer = [UInt8](repeating: 0, count: PipeReader.chunkSize)
        while true {
            let state = stop.state
            if state == .abandoned { return collected }
            let wait = state == .running ? pollInterval : 0
            if isReadable(handle.fileDescriptor, waitingMilliseconds: wait) {
                guard let chunk = PipeReader.next(handle.fileDescriptor, into: &buffer) else { return collected }
                let searchFrom = max(collected.startIndex, collected.endIndex - (terminator?.count ?? 0))
                collected.append(chunk)
                if let terminator, collected[searchFrom...].range(of: terminator) != nil { return collected }
            } else if state == .processExited {
                return collected
            }
        }
    }

    private static func isReadable(_ descriptor: Int32, waitingMilliseconds wait: Int32) -> Bool {
        var request = pollfd(fd: descriptor, events: Int16(POLLIN), revents: 0)
        let ready = poll(&request, 1, wait)
        if ready < 0 { return errno != EINTR }
        return ready > 0 && request.revents != 0
    }
}
