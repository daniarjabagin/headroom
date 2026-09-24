import Foundation

public enum DaemonMethod: String, Sendable, CaseIterable {
    case getState = "GetState"
    case listProviders = "ListProviders"
    case getSettings = "GetSettings"
    case refresh = "Refresh"
    case refreshNow = "RefreshNow"
    case rescan = "Rescan"
    case setSettings = "SetSettings"
    case updateSettings = "UpdateSettings"
    case setAccountLabel = "SetAccountLabel"
    case setAccountOrder = "SetAccountOrder"
    case setAccountHidden = "SetAccountHidden"
    case dismissAccount = "DismissAccount"
    case restoreAccounts = "RestoreAccounts"
    case subscribe = "Subscribe"
}

enum RPCIncoming: Equatable {
    case response(id: Int, error: DaemonError?)
    case notification(method: String)
}

enum RPCCodec {
    static func request(id: Int, method: DaemonMethod, params: [JSONValue]) throws(DaemonError) -> Data {
        let request = RPCRequest(id: id, method: method.rawValue, params: params)
        var line = try encode(request)
        line.append(UInt8(ascii: "\n"))
        return line
    }

    static func encode(_ value: some Encodable) throws(DaemonError) -> Data {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        do {
            return try encoder.encode(value)
        } catch {
            throw .invalidRequest(String(describing: error))
        }
    }

    static func encodeString(_ value: some Encodable) throws(DaemonError) -> String {
        String(decoding: try encode(value), as: UTF8.self)
    }

    static func classify(_ line: Data) -> RPCIncoming? {
        guard let header = try? JSONDecoder().decode(IncomingHeader.self, from: line) else {
            return nil
        }
        if let id = header.id {
            return .response(id: id, error: header.error.map(\.daemonError))
        }
        return header.method.map { .notification(method: $0) }
    }

    static func result<Value: Decodable>(_ type: Value.Type, from line: Data) throws(DaemonError) -> Value {
        try decode(ResultEnvelope<Value>.self, from: line).result
    }

    static func params<Value: Decodable>(_ type: Value.Type, from line: Data) throws(DaemonError) -> Value {
        try decode(ParamsEnvelope<Value>.self, from: line).params
    }

    static func decode<Value: Decodable>(_ type: Value.Type, from data: Data) throws(DaemonError) -> Value {
        do {
            return try JSONDecoder().decode(type, from: data)
        } catch {
            throw .invalidResponse(String(describing: error))
        }
    }
}

private struct RPCRequest: Encodable {
    let jsonrpc = "2.0"
    let id: Int
    let method: String
    let params: [JSONValue]
}

private struct RPCErrorObject: Decodable {
    let code: Int
    let message: String

    var daemonError: DaemonError { .fromRPC(code: code, message: message) }
}

private struct IncomingHeader: Decodable {
    let id: Int?
    let method: String?
    let error: RPCErrorObject?

    enum CodingKeys: String, CodingKey {
        case id, method, error
    }

    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try? container.decodeIfPresent(Int.self, forKey: .id)
        method = try container.decodeIfPresent(String.self, forKey: .method)
        error = try container.decodeIfPresent(RPCErrorObject.self, forKey: .error)
    }
}

private struct ResultEnvelope<Value: Decodable>: Decodable {
    let result: Value
}

private struct ParamsEnvelope<Value: Decodable>: Decodable {
    let params: Value
}

struct StateChangedParams: Decodable {
    let state: DaemonState
}

struct StateChangedHeader: Decodable {
    let state: SchemaHeader
}
