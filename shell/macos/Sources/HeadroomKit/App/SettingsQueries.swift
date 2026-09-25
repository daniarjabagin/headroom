extension DisplaySettings {
    public func isShown(_ section: DisplaySection) -> Bool {
        switch section {
        case .showSpend: showSpend
        case .showAccountSpend: showAccountSpend
        case .showTrend: showTrend
        case .showForecast: showForecast
        }
    }

    public func isHidden(accountID: String, windowID: String) -> Bool {
        hiddenWindows[accountID]?.contains(windowID) ?? false
    }

    public func hiddenWindows(after windowID: String, hidden: Bool, accountID: String) -> [String] {
        let others = (hiddenWindows[accountID] ?? []).filter { $0 != windowID }
        return hidden ? others + [windowID] : others
    }

    public func isStarred(accountID: String) -> Bool {
        starredAccounts.contains(accountID)
    }

    public func starredAccounts(after accountID: String, starred: Bool) -> [String] {
        let others = starredAccounts.filter { $0 != accountID }
        return starred ? others + [accountID] : others
    }

    mutating func set(_ section: DisplaySection, _ value: Bool) {
        switch section {
        case .showSpend: showSpend = value
        case .showAccountSpend: showAccountSpend = value
        case .showTrend: showTrend = value
        case .showForecast: showForecast = value
        }
    }
}

extension NotificationSettings {
    public func isEnabled(_ milestone: Milestone) -> Bool {
        switch milestone {
        case .almostOut: almostOut
        case .cuttingItClose: cuttingItClose
        case .willRunOut: willRunOut
        case .reset: reset
        }
    }

    mutating func set(_ milestone: Milestone, _ value: Bool) {
        switch milestone {
        case .almostOut: almostOut = value
        case .cuttingItClose: cuttingItClose = value
        case .willRunOut: willRunOut = value
        case .reset: reset = value
        }
    }
}

extension SettingsChange {
    public var requiresRelease06: Bool {
        switch self {
        case .panelLabel(let label): label == .none
        case .theme, .language, .valueMode, .resetFormat, .translucent, .combineAccounts, .section, .reducedMotion,
            .refreshInterval, .headline, .notification, .hiddenWindows:
            false
        case .density, .timeFormat, .panel, .spend, .starredAccounts, .collapseUnstarred, .hideOnScreenShare,
            .adaptiveRefresh, .alerts, .statusPages, .shortcut, .logLevel, .onboardingCompleted:
            true
        }
    }
}

extension DaemonFeatures {
    public func allows(_ change: SettingsChange) -> Bool {
        release06 || !change.requiresRelease06
    }
}
