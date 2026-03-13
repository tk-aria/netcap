import Foundation

/// NetcapBridge - iOS bridge for the netcap FFI library.
///
/// This class wraps the UniFFI-generated Swift bindings to provide
/// a clean iOS-friendly API for the netcap proxy functionality.
///
/// Usage:
/// ```swift
/// let bridge = NetcapBridge(port: 8080, storagePath: documentsPath)
/// try bridge.start()
/// let stats = try bridge.getStats()
/// try bridge.stop()
/// ```
public class NetcapBridge {
    private let port: UInt16
    private let storagePath: String
    private let includeDomains: [String]
    private let excludeDomains: [String]
    private var isRunning = false

    public init(
        port: UInt16 = 8080,
        storagePath: String,
        includeDomains: [String] = [],
        excludeDomains: [String] = []
    ) {
        self.port = port
        self.storagePath = storagePath
        self.includeDomains = includeDomains
        self.excludeDomains = excludeDomains
    }

    /// Start the proxy server.
    public func start() throws {
        guard !isRunning else {
            throw NetcapError.alreadyRunning
        }
        // TODO: Call UniFFI-generated NetcapProxy.start()
        isRunning = true
    }

    /// Stop the proxy server.
    public func stop() throws {
        guard isRunning else {
            throw NetcapError.notRunning
        }
        // TODO: Call UniFFI-generated NetcapProxy.stop()
        isRunning = false
    }

    /// Get the CA certificate in PEM format.
    public func getCaCertificatePem() throws -> String {
        // TODO: Call UniFFI-generated NetcapProxy.getCaCertificatePem()
        return ""
    }

    /// Get current capture statistics.
    public func getStats() throws -> CaptureStats {
        // TODO: Call UniFFI-generated NetcapProxy.getStats()
        return CaptureStats(totalRequests: 0, totalResponses: 0, activeConnections: 0, bytesCaptured: 0)
    }

    /// Get captured events as JSON string.
    public func getCaptureEvents(offset: UInt64 = 0, limit: UInt64 = 100) throws -> String {
        // TODO: Call UniFFI-generated NetcapProxy.getCaptureEvents()
        return "[]"
    }

    public var proxyRunning: Bool { isRunning }
}

/// Capture statistics from the proxy.
public struct CaptureStats {
    public let totalRequests: UInt64
    public let totalResponses: UInt64
    public let activeConnections: UInt32
    public let bytesCaptured: UInt64
}

/// Errors from the netcap proxy.
public enum NetcapError: Error {
    case initFailed(String)
    case proxyError(String)
    case alreadyRunning
    case notRunning
    case storageError(String)
    case certError(String)
}
