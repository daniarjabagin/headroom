import Foundation

#if canImport(Darwin)
    import Darwin
#elseif canImport(Glibc)
    import Glibc
#endif

enum POSIXSocket {
    static func connect(path: String) throws(DaemonError) -> Int32 {
        var address = try address(for: path)
        let descriptor = socket(AF_UNIX, streamType, 0)
        guard descriptor >= 0 else { throw failure("socket") }
        disableSigpipe(descriptor)
        let length = socklen_t(MemoryLayout<sockaddr_un>.size)
        let result = withUnsafePointer(to: &address) { pointer in
            pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) { socketAddress in
                systemConnect(descriptor, socketAddress, length)
            }
        }
        guard result == 0 else {
            let error = failure("connect \(path)")
            closeDescriptor(descriptor)
            throw error
        }
        return descriptor
    }

    static func address(for path: String) throws(DaemonError) -> sockaddr_un {
        var address = sockaddr_un()
        address.sun_family = sa_family_t(AF_UNIX)
        let bytes = Array(path.utf8)
        let capacity = MemoryLayout.size(ofValue: address.sun_path)
        guard !bytes.isEmpty, bytes.count < capacity else { throw .socketPathTooLong(path) }
        withUnsafeMutableBytes(of: &address.sun_path) { buffer in
            buffer.copyBytes(from: bytes)
        }
        #if canImport(Darwin)
            address.sun_len = UInt8(MemoryLayout<sockaddr_un>.size)
        #endif
        return address
    }

    static func send(_ descriptor: Int32, _ data: Data) throws(DaemonError) {
        var offset = 0
        while offset < data.count {
            let written = data.withUnsafeBytes { buffer in
                systemSend(descriptor, buffer.baseAddress.map { $0 + offset }, data.count - offset)
            }
            if written < 0 && errno == EINTR { continue }
            guard written > 0 else { throw failure("send") }
            offset += written
        }
    }

    static func receive(_ descriptor: Int32, into buffer: inout [UInt8]) -> Int {
        buffer.withUnsafeMutableBytes { raw in
            recv(descriptor, raw.baseAddress, raw.count, 0)
        }
    }

    static func shutdownBoth(_ descriptor: Int32) {
        _ = shutdown(descriptor, Int32(SHUT_RDWR))
    }

    static func closeDescriptor(_ descriptor: Int32) {
        #if canImport(Darwin)
            _ = Darwin.close(descriptor)
        #else
            _ = Glibc.close(descriptor)
        #endif
    }

    static func failure(_ operation: String) -> DaemonError {
        .transport("\(operation): \(String(cString: strerror(errno)))")
    }

    private static var streamType: Int32 {
        #if canImport(Darwin)
            SOCK_STREAM
        #else
            Int32(SOCK_STREAM.rawValue)
        #endif
    }

    private static func systemConnect(
        _ descriptor: Int32, _ address: UnsafePointer<sockaddr>, _ length: socklen_t
    ) -> Int32 {
        #if canImport(Darwin)
            Darwin.connect(descriptor, address, length)
        #else
            Glibc.connect(descriptor, address, length)
        #endif
    }

    private static func systemSend(_ descriptor: Int32, _ bytes: UnsafeRawPointer?, _ count: Int) -> Int {
        #if canImport(Darwin)
            Darwin.send(descriptor, bytes, count, 0)
        #else
            Glibc.send(descriptor, bytes, count, Int32(MSG_NOSIGNAL))
        #endif
    }

    private static func disableSigpipe(_ descriptor: Int32) {
        #if canImport(Darwin)
            var enabled: Int32 = 1
            _ = setsockopt(
                descriptor, SOL_SOCKET, SO_NOSIGPIPE, &enabled, socklen_t(MemoryLayout<Int32>.size))
        #endif
    }
}
