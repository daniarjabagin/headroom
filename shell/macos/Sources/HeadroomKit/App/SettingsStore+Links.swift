extension ProviderList {
    public func links(for provider: String) -> ProviderLinks? {
        guard case .loaded(let providers) = self else { return nil }
        return providers.first { $0.id == provider }?.links
    }
}

extension SettingsStore {
    public func links(for provider: String) -> ProviderLinks? {
        providers.links(for: provider)
    }
}
