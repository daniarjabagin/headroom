public enum DaemonError: Error, Sendable, Hashable {
    case notConnected
    case disconnected
    case parseError(String)
    case invalidRequest(String)
    case unknownMethod(String)
    case invalidArguments(String)
    case daemonFailure(String)
    case rpc(code: Int, message: String)
    case invalidResponse(String)
    case unsupportedSchema(Int)
    case transport(String)
    case socketPathTooLong(String)
    case lineTooLong

    public static func fromRPC(code: Int, message: String) -> DaemonError {
        switch code {
        case -32700: .parseError(message)
        case -32600: .invalidRequest(message)
        case -32601: .unknownMethod(message)
        case -32602: .invalidArguments(message)
        case -32000: .daemonFailure(message)
        default: .rpc(code: code, message: message)
        }
    }

    public static func wrapping(_ error: any Error) -> DaemonError {
        if let daemonError = error as? DaemonError { return daemonError }
        return .transport(String(describing: error))
    }
}
