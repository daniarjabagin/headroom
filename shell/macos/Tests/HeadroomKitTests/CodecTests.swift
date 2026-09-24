import Foundation
import XCTest

@testable import HeadroomKit

final class RFC3339Tests: XCTestCase {
    func testParsesUTCAndOffsets() throws {
        let utc = try XCTUnwrap(RFC3339.parse("2026-09-23T10:00:00Z"))
        XCTAssertEqual(utc.timeIntervalSince1970, 1_790_157_600)
        XCTAssertEqual(RFC3339.parse("2026-09-23T15:00:00+05:00"), utc)
        XCTAssertEqual(RFC3339.parse("2026-09-23T05:30:00-04:30"), utc)
        XCTAssertEqual(RFC3339.parse("1970-01-01T00:00:00Z")?.timeIntervalSince1970, 0)
        XCTAssertEqual(RFC3339.parse("2000-02-29T00:00:00z")?.timeIntervalSince1970, 951_782_400)
    }

    func testParsesFractionalSeconds() throws {
        let date = try XCTUnwrap(RFC3339.parse("2026-09-23T10:23:28.695652174Z"))
        XCTAssertEqual(date.timeIntervalSince1970, 1_790_159_008.695652174, accuracy: 1e-6)
    }

    func testRejectsMalformedText() {
        for text in [
            "", "2026-09-23", "2026-09-23T10:00:00", "2026-13-01T00:00:00Z", "2026-09-23T25:00:00Z",
            "2026-09-23T10:00:00Zjunk", "2026-09-23T10:00:00+0500",
        ] {
            XCTAssertNil(RFC3339.parse(text), text)
        }
    }
}

final class LineBufferTests: XCTestCase {
    func testSplitsLinesAcrossChunks() throws {
        var buffer = LineBuffer()
        XCTAssertEqual(try buffer.append(Data("{\"a\":".utf8)), [])
        let lines = try buffer.append(Data("1}\n{\"b\":2}\r\n\n{\"c\"".utf8))
        XCTAssertEqual(lines.map { String(decoding: $0, as: UTF8.self) }, [#"{"a":1}"#, #"{"b":2}"#])
        XCTAssertEqual(try buffer.append(Data(":3}\n".utf8)).count, 1)
    }

    func testRejectsOverlongLine() {
        var buffer = LineBuffer()
        let chunk = Data(repeating: UInt8(ascii: "x"), count: LineBuffer.maximumLineLength + 1)
        XCTAssertThrowsError(try buffer.append(chunk)) { error in
            XCTAssertEqual(error as? DaemonError, .lineTooLong)
        }
    }
}

final class RPCCodecTests: XCTestCase {
    func testRequestIsOneCompactLine() throws {
        let line = try RPCCodec.request(
            id: 7, method: .setAccountHidden, params: [.string("codex:1/a"), .bool(true)])
        XCTAssertEqual(
            String(decoding: line, as: UTF8.self),
            #"{"id":7,"jsonrpc":"2.0","method":"SetAccountHidden","params":["codex:1/a",true]}"# + "\n")
    }

    func testClassifiesResponsesAndNotifications() {
        XCTAssertEqual(RPCCodec.classify(Data(Reply.result(3, "null").utf8)), .response(id: 3, error: nil))
        XCTAssertEqual(
            RPCCodec.classify(Data(Reply.error(4, code: -32602, message: "unknown account id").utf8)),
            .response(id: 4, error: .invalidArguments("unknown account id")))
        XCTAssertEqual(
            RPCCodec.classify(Data(Reply.notification("OpenRequested", "{}").utf8)),
            .notification(method: "OpenRequested"))
        XCTAssertNil(RPCCodec.classify(Data("not json".utf8)))
    }

    func testErrorCodesMapToTypedErrors() {
        XCTAssertEqual(DaemonError.fromRPC(code: -32700, message: "m"), .parseError("m"))
        XCTAssertEqual(DaemonError.fromRPC(code: -32600, message: "m"), .invalidRequest("m"))
        XCTAssertEqual(DaemonError.fromRPC(code: -32601, message: "m"), .unknownMethod("m"))
        XCTAssertEqual(DaemonError.fromRPC(code: -32602, message: "m"), .invalidArguments("m"))
        XCTAssertEqual(DaemonError.fromRPC(code: -32000, message: "m"), .daemonFailure("m"))
        XCTAssertEqual(DaemonError.fromRPC(code: 12, message: "m"), .rpc(code: 12, message: "m"))
    }
}

final class SocketPathTests: XCTestCase {
    func testDefaultPathUnderApplicationSupport() {
        XCTAssertEqual(
            SocketPath.resolve(environment: [:], home: "/Users/ada", uid: 501),
            "/Users/ada/Library/Application Support/Headroom/daemon.sock")
    }

    func testOverrideWins() {
        XCTAssertEqual(
            SocketPath.resolve(environment: ["HEADROOM_SOCKET": "/tmp/h.sock"], home: "/Users/ada", uid: 501),
            "/tmp/h.sock")
    }

    func testLongHomeFallsBackToTemporaryDirectory() {
        let home = "/Users/" + String(repeating: "a", count: 60)
        let environment = ["TMPDIR": "/var/folders/xy/T/"]
        XCTAssertEqual(
            SocketPath.resolve(environment: environment, home: home, uid: 501),
            "/var/folders/xy/T/headroom-501/daemon.sock")
        XCTAssertEqual(SocketPath.resolve(environment: [:], home: home, uid: 7), "/tmp/headroom-7/daemon.sock")
    }

    func testFallbackNeverCreatesTheDirectory() throws {
        let temporary = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let home = "/Users/" + String(repeating: "a", count: 60)
        let path = SocketPath.resolve(environment: ["TMPDIR": temporary.path], home: home, uid: 501)
        XCTAssertEqual(path, temporary.path + "/headroom-501/daemon.sock")
        XCTAssertFalse(FileManager.default.fileExists(atPath: temporary.path))
    }

    func testBoundaryLengthIs103Bytes() {
        let suffix = "/Library/Application Support/Headroom/daemon.sock"
        let fits = "/" + String(repeating: "h", count: 102 - suffix.utf8.count)
        XCTAssertEqual(SocketPath.resolve(environment: [:], home: fits, uid: 1).utf8.count, 103)
        XCTAssertEqual(SocketPath.resolve(environment: [:], home: fits + "h", uid: 1), "/tmp/headroom-1/daemon.sock")
    }
}
