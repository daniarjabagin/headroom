#if canImport(AppKit)
    import Carbon.HIToolbox
    import HeadroomKit
    import os

    @MainActor
    final class HotKey {
        private static let signature: OSType = 0x4864_526D

        var onPress: (@MainActor () -> Void)?

        private let log = Logger(subsystem: "io.github.daniarjabagin.headroom", category: "hotkey")
        private var chord: HotKeyChord?
        private var registration: EventHotKeyRef?
        private var handler: EventHandlerRef?

        func register(_ wanted: HotKeyChord?) {
            guard wanted != chord else { return }
            unregister()
            chord = wanted
            guard let wanted, installHandler() else { return }
            var reference: EventHotKeyRef?
            let status = RegisterEventHotKey(
                wanted.carbonKeyCode, wanted.carbonModifiers, EventHotKeyID(signature: Self.signature, id: 1),
                GetApplicationEventTarget(), 0, &reference)
            guard status == noErr else {
                log.error("global shortcut not registered: \(status, privacy: .public)")
                return
            }
            registration = reference
        }

        fileprivate func press() {
            onPress?()
        }

        private func unregister() {
            if let registration { UnregisterEventHotKey(registration) }
            registration = nil
        }

        private func installHandler() -> Bool {
            guard handler == nil else { return true }
            var pressed = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
            let context = Unmanaged.passUnretained(self).toOpaque()
            let status = InstallEventHandler(GetApplicationEventTarget(), hotKeyPressed, 1, &pressed, context, &handler)
            guard status == noErr else {
                log.error("global shortcut handler not installed: \(status, privacy: .public)")
                return false
            }
            return true
        }
    }

    private func hotKeyPressed(
        _ call: EventHandlerCallRef?, _ event: EventRef?, _ context: UnsafeMutableRawPointer?
    ) -> OSStatus {
        guard let context else { return OSStatus(eventNotHandledErr) }
        let address = UInt(bitPattern: context)
        MainActor.assumeIsolated {
            guard let pointer = UnsafeMutableRawPointer(bitPattern: address) else { return }
            Unmanaged<HotKey>.fromOpaque(pointer).takeUnretainedValue().press()
        }
        return noErr
    }
#endif
