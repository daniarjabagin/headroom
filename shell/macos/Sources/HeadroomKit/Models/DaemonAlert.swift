public struct DaemonAlert: Decodable, Sendable, Hashable, Identifiable {
    public let id: String
    public let title: String
    public let body: String
    public let accountID: String?
    public let urgency: AlertUrgency

    enum CodingKeys: String, CodingKey {
        case id, title, body, urgency
        case accountID = "account_id"
    }
}
