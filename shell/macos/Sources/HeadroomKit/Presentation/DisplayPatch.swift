public enum DisplayPatch {
    public static func toggledValueMode(_ display: DisplaySettings) -> [String: JSONValue] {
        let next: ValueMode = display.valueMode == .left ? .used : .left
        return patch("value_mode", next.rawValue)
    }

    public static func toggledResetFormat(_ display: DisplaySettings) -> [String: JSONValue] {
        let next: ResetFormat = display.resetFormat == .countdown ? .exact : .countdown
        return patch("reset_format", next.rawValue)
    }

    private static func patch(_ key: String, _ value: String) -> [String: JSONValue] {
        ["display": .object([key: .string(value)])]
    }
}
