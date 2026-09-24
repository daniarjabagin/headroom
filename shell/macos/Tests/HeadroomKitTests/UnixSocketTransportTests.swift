import Foundation
import XCTest

@testable import HeadroomKit

#if canImport(Darwin)
    import Darwin
#elseif canImport(Glibc)
    import Glibc
#endif

final class UnixSocketTransportTests: XCTestCase {
    func testRoundTripOverRealSocket() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
            "hr-\(getpid())-\(UInt32.random(in: 0...UInt32.max))")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let path = directory.appendingPathComponent("d.sock").path
        let server = try EchoServer(path: path)
        let connection = try await UnixSocketTransport(path: path).open()
        try await connection.send(Data("{\"ping\":1}\n".utf8))
        var iterator = connection.lines.makeAsyncIterator()
        let reply = try await iterator.next()
        XCTAssertEqual(reply.map { String(decoding: $0, as: UTF8.self) }, #"{"ping":1}"#)
        await connection.close()
        let end = try await iterator.next()
        XCTAssertNil(end)
        server.stop()
    }

    func testConnectToMissingSocketFails() async {
        do {
            _ = try await UnixSocketTransport(path: "/nonexistent/headroom.sock").open()
            XCTFail("expected failure")
        } catch {
            guard case .transport = error as? DaemonError else { return XCTFail("\(error)") }
        }
    }

    func testOverlongPathIsRejected() {
        let path = "/" + String(repeating: "p", count: 200)
        XCTAssertThrowsError(try POSIXSocket.address(for: path)) { error in
            XCTAssertEqual(error as? DaemonError, .socketPathTooLong(path))
        }
    }
}

private final class EchoServer: @unchecked Sendable {
    private let listener: Int32

    init(path: String) throws {
        var address = try POSIXSocket.address(for: path)
        #if canImport(Darwin)
            listener = socket(AF_UNIX, SOCK_STREAM, 0)
        #else
            listener = socket(AF_UNIX, Int32(SOCK_STREAM.rawValue), 0)
        #endif
        let bound = withUnsafePointer(to: &address) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                bind(listener, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
            }
        }
        guard bound == 0, listen(listener, 1) == 0 else { throw POSIXSocket.failure("listen") }
        Thread { [listener] in Self.serveOne(listener) }.start()
    }

    func stop() {
        POSIXSocket.shutdownBoth(listener)
        POSIXSocket.closeDescriptor(listener)
    }

    private static func serveOne(_ listener: Int32) {
        let client = accept(listener, nil, nil)
        guard client >= 0 else { return }
        var buffer = [UInt8](repeating: 0, count: 1024)
        while true {
            let count = POSIXSocket.receive(client, into: &buffer)
            guard count > 0 else { break }
            try? POSIXSocket.send(client, Data(buffer[0..<count]))
        }
        POSIXSocket.closeDescriptor(client)
    }
}
