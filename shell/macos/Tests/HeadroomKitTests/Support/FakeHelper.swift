import Foundation

@testable import HeadroomKit

final class FakeHelperHandle: HelperProcessHandle, @unchecked Sendable {
    let output: AsyncStream<HelperOutput>

    private let feed: AsyncStream<HelperOutput>.Continuation
    private let lock = NSLock()
    private var written: [String] = []
    private var closed = false
    private var cancelled = false

    init() {
        (output, feed) = AsyncStream.makeStream(of: HelperOutput.self)
    }

    var lines: [String] { lock.withLock { written } }
    var inputClosed: Bool { lock.withLock { closed } }
    var wasCancelled: Bool { lock.withLock { cancelled } }

    func emit(_ values: HelperOutput...) {
        for value in values { feed.yield(value) }
    }

    func end() {
        feed.finish()
    }

    func write(_ line: String) {
        lock.withLock { written.append(line) }
    }

    func closeInput() {
        lock.withLock { closed = true }
    }

    func cancel() {
        lock.withLock { cancelled = true }
        feed.finish()
    }
}

final class FakeHelperLauncher: HelperLaunching, @unchecked Sendable {
    let handle = FakeHelperHandle()

    private let lock = NSLock()
    private let failure: HelperFailure?
    private var launched: [AccountCommand] = []

    init(failure: HelperFailure? = nil) {
        self.failure = failure
    }

    var commands: [AccountCommand] { lock.withLock { launched } }

    func launch(_ command: AccountCommand) throws(HelperFailure) -> any HelperProcessHandle {
        lock.withLock { launched.append(command) }
        if let failure { throw failure }
        return handle
    }
}
