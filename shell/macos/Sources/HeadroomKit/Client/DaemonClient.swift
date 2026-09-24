import Foundation

public enum DaemonEvent: Sendable, Equatable {
    case connecting
    case connected
    case disconnected(DaemonError?)
    case state(DaemonState)
    case alert(DaemonAlert)
    case openRequested
    case failure(DaemonError)
}

public actor DaemonClient {
    public nonisolated let events: AsyncStream<DaemonEvent>

    private let emitter: AsyncStream<DaemonEvent>.Continuation
    private let transport: any LineTransport
    private let backoff: Backoff
    private let sleeper: any Sleeper
    private var connection: (any LineConnection)?
    private var pending: [Int: CheckedContinuation<Data, any Error>] = [:]
    private var nextID = 1
    private var loop: Task<Void, Never>?

    public init(transport: any LineTransport, backoff: Backoff = .reconnect, sleeper: any Sleeper = TaskSleeper()) {
        self.transport = transport
        self.backoff = backoff
        self.sleeper = sleeper
        (events, emitter) = AsyncStream.makeStream(of: DaemonEvent.self, bufferingPolicy: .bufferingNewest(64))
    }

    public func start() {
        guard loop == nil else { return }
        loop = Task { await self.run() }
    }

    public func stop() async {
        let running = loop
        loop = nil
        running?.cancel()
        await connection?.close()
        await running?.value
    }

    func call(_ method: DaemonMethod, _ params: [JSONValue] = []) async throws(DaemonError) -> Data {
        guard let connection else { throw .notConnected }
        let id = nextID
        nextID += 1
        let line = try RPCCodec.request(id: id, method: method, params: params)
        do {
            return try await withCheckedThrowingContinuation { continuation in
                pending[id] = continuation
                Task { await self.transmit(line, id: id, over: connection) }
            }
        } catch {
            throw DaemonError.wrapping(error)
        }
    }

    private func transmit(_ line: Data, id: Int, over connection: any LineConnection) async {
        do {
            try await connection.send(line)
        } catch {
            pending.removeValue(forKey: id)?.resume(throwing: DaemonError.wrapping(error))
        }
    }

    private func run() async {
        var failures = 0
        while !Task.isCancelled {
            failures = await runSession() ? 0 : failures + 1
            guard !Task.isCancelled else { break }
            try? await sleeper.sleep(for: backoff.delay(attempt: max(0, failures - 1)))
        }
    }

    private func runSession() async -> Bool {
        emitter.yield(.connecting)
        let opened: any LineConnection
        do {
            opened = try await transport.open()
        } catch {
            emitter.yield(.disconnected(DaemonError.wrapping(error)))
            return false
        }
        connection = opened
        if Task.isCancelled { await opened.close() }
        let reader = Task { await self.consume(opened) }
        emitter.yield(.connected)
        await bootstrap(opened)
        emitter.yield(.disconnected(await reader.value))
        return true
    }

    private func bootstrap(_ opened: any LineConnection) async {
        do {
            _ = try await call(.subscribe)
            emitter.yield(.state(try await getState()))
        } catch {
            emitter.yield(.failure(DaemonError.wrapping(error)))
            await opened.close()
        }
    }

    private func consume(_ opened: any LineConnection) async -> DaemonError? {
        let ending = await readLines(opened)
        connection = nil
        failPending()
        return ending
    }

    private func readLines(_ opened: any LineConnection) async -> DaemonError? {
        do {
            for try await line in opened.lines { handle(line) }
            return nil
        } catch {
            return DaemonError.wrapping(error)
        }
    }

    private func failPending() {
        let waiting = pending
        pending.removeAll()
        for continuation in waiting.values { continuation.resume(throwing: DaemonError.disconnected) }
    }

    private func handle(_ line: Data) {
        switch RPCCodec.classify(line) {
        case .response(let id, let error):
            guard let continuation = pending.removeValue(forKey: id) else { return }
            if let error { continuation.resume(throwing: error) } else { continuation.resume(returning: line) }
        case .notification(let method):
            handleNotification(method, line)
        case nil:
            emitter.yield(.failure(.invalidResponse("unreadable line from daemon")))
        }
    }

    private func handleNotification(_ method: String, _ line: Data) {
        do {
            switch method {
            case "StateChanged": emitter.yield(.state(try Self.stateFromNotification(line)))
            case "Alert": emitter.yield(.alert(try RPCCodec.params(DaemonAlert.self, from: line)))
            case "OpenRequested": emitter.yield(.openRequested)
            default: return
            }
        } catch {
            emitter.yield(.failure(DaemonError.wrapping(error)))
        }
    }

    static func stateFromNotification(_ line: Data) throws(DaemonError) -> DaemonState {
        let header = try RPCCodec.params(StateChangedHeader.self, from: line)
        try checkSchema(header.state.version)
        return try RPCCodec.params(StateChangedParams.self, from: line).state
    }

    static func checkSchema(_ version: Int) throws(DaemonError) {
        guard version == DaemonState.schemaVersion else { throw .unsupportedSchema(version) }
    }
}
