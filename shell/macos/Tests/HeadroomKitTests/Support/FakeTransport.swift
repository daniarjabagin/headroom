import Foundation

@testable import HeadroomKit

struct FakeRequest: Decodable, Sendable {
    let id: Int
    let method: String
    let params: [JSONValue]
}

typealias Responder = @Sendable (FakeRequest) -> [String]

final class FakeConnection: LineConnection, @unchecked Sendable {
    let lines: AsyncThrowingStream<Data, any Error>

    private let feed: AsyncThrowingStream<Data, any Error>.Continuation
    private let responder: Responder
    private let lock = NSLock()
    private var recorded: [FakeRequest] = []

    init(responder: @escaping Responder) {
        self.responder = responder
        (lines, feed) = AsyncThrowingStream.makeStream(of: Data.self)
    }

    var requests: [FakeRequest] { lock.withLock { recorded } }

    func send(_ line: Data) async throws {
        let request = try JSONDecoder().decode(FakeRequest.self, from: line)
        lock.withLock { recorded.append(request) }
        for reply in responder(request) { deliver(reply) }
    }

    func deliver(_ json: String) {
        feed.yield(Data(json.utf8))
    }

    func drop() {
        feed.finish()
    }

    func close() async {
        feed.finish()
    }
}

final class FakeTransport: LineTransport, @unchecked Sendable {
    private let lock = NSLock()
    private let responder: Responder
    private var failuresLeft: Int
    private var opened: [FakeConnection] = []

    init(failFirst failures: Int = 0, responder: @escaping Responder) {
        self.responder = responder
        failuresLeft = failures
    }

    var connections: [FakeConnection] { lock.withLock { opened } }

    func open() async throws -> any LineConnection {
        try lock.withLock {
            if failuresLeft > 0 {
                failuresLeft -= 1
                throw DaemonError.transport("connection refused")
            }
            let connection = FakeConnection(responder: responder)
            opened.append(connection)
            return connection
        }
    }
}

final class RecordingSleeper: Sleeper, @unchecked Sendable {
    private let lock = NSLock()
    private var recorded: [Duration] = []

    var durations: [Duration] { lock.withLock { recorded } }

    func sleep(for duration: Duration) async throws {
        lock.withLock { recorded.append(duration) }
        await Task.yield()
        try Task.checkCancellation()
    }
}

enum Reply {
    static func result(_ id: Int, _ json: String) -> String {
        #"{"jsonrpc":"2.0","id":\#(id),"result":\#(json)}"#
    }

    static func error(_ id: Int, code: Int, message: String) -> String {
        #"{"jsonrpc":"2.0","id":\#(id),"error":{"code":\#(code),"message":"\#(message)"}}"#
    }

    static func notification(_ method: String, _ params: String) -> String {
        #"{"jsonrpc":"2.0","method":"\#(method)","params":\#(params)}"#
    }

    static func daemon(state: String) -> Responder {
        { request in
            switch request.method {
            case "GetState": [result(request.id, state)]
            default: [result(request.id, "null")]
            }
        }
    }
}

func collect(
    _ events: AsyncStream<DaemonEvent>, until done: @escaping @Sendable ([DaemonEvent]) -> Bool
) async -> [DaemonEvent] {
    var seen: [DaemonEvent] = []
    for await event in events {
        seen.append(event)
        if done(seen) { break }
    }
    return seen
}
