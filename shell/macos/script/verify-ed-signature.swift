import CryptoKit
import Foundation

func fail(_ message: String, status: Int32) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(status)
}

let arguments = Array(CommandLine.arguments.dropFirst())
guard arguments.count == 3 else {
    fail("usage: verify-ed-signature.swift <public-key-base64> <signature-base64> <file>", status: 2)
}
guard let publicKeyData = Data(base64Encoded: arguments[0]), let signature = Data(base64Encoded: arguments[1])
else {
    fail("the public key or the signature is not valid base64", status: 2)
}
guard let file = FileManager.default.contents(atPath: arguments[2]) else {
    fail("cannot read \(arguments[2])", status: 2)
}
guard let publicKey = try? Curve25519.Signing.PublicKey(rawRepresentation: publicKeyData) else {
    fail("the public key is not a 32-byte Ed25519 key", status: 2)
}
guard publicKey.isValidSignature(signature, for: file) else {
    fail("the EdDSA signature of \(arguments[2]) does not match the public key", status: 1)
}
print("EdDSA signature of \(arguments[2]) matches the public key")
