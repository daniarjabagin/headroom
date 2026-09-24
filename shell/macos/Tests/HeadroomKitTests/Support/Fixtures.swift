import Foundation
import XCTest

@testable import HeadroomKit

enum Fixture {
    static func data(_ name: String) throws -> Data {
        let url = try XCTUnwrap(Bundle.module.url(forResource: name, withExtension: "json", subdirectory: "Fixtures"))
        return try Data(contentsOf: url)
    }

    static func text(_ name: String) throws -> String {
        String(decoding: try data(name), as: UTF8.self)
    }

    static func decode<Value: Decodable>(_ type: Value.Type, _ name: String) throws -> Value {
        try JSONDecoder().decode(type, from: data(name))
    }

    static func decode<Value: Decodable>(_ type: Value.Type, json: String) throws -> Value {
        try JSONDecoder().decode(type, from: Data(json.utf8))
    }

    static func timestamp(_ text: String) throws -> Timestamp {
        try XCTUnwrap(Timestamp(rfc3339: text))
    }
}
