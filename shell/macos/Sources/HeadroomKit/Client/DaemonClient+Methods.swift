import Foundation

extension DaemonClient {
    public func getState() async throws(DaemonError) -> DaemonState {
        let line = try await call(.getState)
        try Self.checkSchema(try RPCCodec.result(SchemaHeader.self, from: line).version)
        return try RPCCodec.result(DaemonState.self, from: line)
    }

    public func listProviders() async throws(DaemonError) -> ProvidersPayload {
        try RPCCodec.result(ProvidersPayload.self, from: try await call(.listProviders))
    }

    public func getSettings() async throws(DaemonError) -> Settings {
        try RPCCodec.result(Settings.self, from: try await call(.getSettings))
    }

    public func refresh(accountID: String = "") async throws(DaemonError) {
        _ = try await call(.refresh, [.string(accountID)])
    }

    public func refreshNow() async throws(DaemonError) {
        _ = try await call(.refreshNow)
    }

    public func rescan() async throws(DaemonError) {
        _ = try await call(.rescan)
    }

    public func updateSettings(_ patch: [String: JSONValue]) async throws(DaemonError) {
        _ = try await call(.updateSettings, [.string(try RPCCodec.encodeString(patch))])
    }

    public func resetSettings() async throws(DaemonError) {
        _ = try await call(.resetSettings)
    }

    public func getSpend(_ query: SpendQuery) async throws(DaemonError) -> SpendReport {
        let line = try await call(.getSpend, [.string(try RPCCodec.encodeString(query))])
        return try RPCCodec.result(SpendReport.self, from: line)
    }

    public func getDiagnostics() async throws(DaemonError) -> Diagnostics {
        try RPCCodec.result(Diagnostics.self, from: try await call(.getDiagnostics))
    }

    public func setAccountLabel(accountID: String, label: String) async throws(DaemonError) {
        _ = try await call(.setAccountLabel, [.string(accountID), .string(label)])
    }

    public func setAccountOrder(_ accountIDs: [String]) async throws(DaemonError) {
        _ = try await call(.setAccountOrder, [.array(accountIDs.map(JSONValue.string))])
    }

    public func setAccountHidden(accountID: String, hidden: Bool) async throws(DaemonError) {
        _ = try await call(.setAccountHidden, [.string(accountID), .bool(hidden)])
    }

    public func dismissAccount(accountID: String) async throws(DaemonError) {
        _ = try await call(.dismissAccount, [.string(accountID)])
    }

    public func restoreAccounts(provider: String = "") async throws(DaemonError) {
        _ = try await call(.restoreAccounts, [.string(provider)])
    }
}
