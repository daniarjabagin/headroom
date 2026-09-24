import Foundation

public protocol LineConnection: Sendable {
    var lines: AsyncThrowingStream<Data, any Error> { get }
    func send(_ line: Data) async throws
    func close() async
}

public protocol LineTransport: Sendable {
    func open() async throws -> any LineConnection
}
