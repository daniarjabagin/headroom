public enum DisplayToggle {
    public static func valueMode(_ display: DisplaySettings) -> SettingsChange {
        .valueMode(display.valueMode == .left ? .used : .left)
    }

    public static func resetFormat(_ display: DisplaySettings) -> SettingsChange {
        .resetFormat(display.resetFormat == .countdown ? .exact : .countdown)
    }
}

public enum ReadingTips {
    public static func value(_ display: DisplaySettings, strings: UIStrings) -> String {
        strings.text(display.valueMode == .left ? PopupExtraText.clickShowUsed : .clickShowLeft)
    }

    public static func reset(_ display: DisplaySettings, strings: UIStrings) -> String {
        strings.text(display.resetFormat == .countdown ? PopupExtraText.clickShowResetTime : .clickShowCountdown)
    }
}
