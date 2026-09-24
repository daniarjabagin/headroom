#if canImport(AppKit)
    import AppKit

    _ = signal(SIGPIPE, SIG_IGN)
    let application = NSApplication.shared
    let delegate = AppDelegate()
    application.delegate = delegate
    application.setActivationPolicy(.accessory)
    application.run()
#endif
