public enum DisplayToggle {
    public static func valueMode(_ display: DisplaySettings) -> SettingsChange {
        .valueMode(display.valueMode == .left ? .used : .left)
    }

    public static func resetFormat(_ display: DisplaySettings) -> SettingsChange {
        .resetFormat(display.resetFormat == .countdown ? .exact : .countdown)
    }
}
