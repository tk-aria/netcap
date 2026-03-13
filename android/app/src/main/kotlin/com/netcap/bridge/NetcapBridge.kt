package com.netcap.bridge

/**
 * NetcapBridge - Android bridge for the netcap FFI library.
 *
 * This class wraps the UniFFI-generated Kotlin bindings to provide
 * a clean Android-friendly API for the netcap proxy functionality.
 *
 * Usage:
 * ```kotlin
 * val bridge = NetcapBridge(port = 8080, storagePath = context.filesDir.absolutePath)
 * bridge.start()
 * val stats = bridge.getStats()
 * bridge.stop()
 * ```
 */
class NetcapBridge(
    private val port: Int = 8080,
    private val storagePath: String,
    private val includeDomains: List<String> = emptyList(),
    private val excludeDomains: List<String> = emptyList()
) {
    private var isRunning = false

    /**
     * Start the proxy server.
     * @throws IllegalStateException if the proxy is already running
     */
    fun start() {
        if (isRunning) {
            throw IllegalStateException("Proxy is already running")
        }
        // TODO: Call UniFFI-generated NetcapProxy.start()
        isRunning = true
    }

    /**
     * Stop the proxy server.
     * @throws IllegalStateException if the proxy is not running
     */
    fun stop() {
        if (!isRunning) {
            throw IllegalStateException("Proxy is not running")
        }
        // TODO: Call UniFFI-generated NetcapProxy.stop()
        isRunning = false
    }

    /**
     * Get the CA certificate in PEM format.
     * This certificate needs to be installed on the device to intercept HTTPS traffic.
     */
    fun getCaCertificatePem(): String {
        // TODO: Call UniFFI-generated NetcapProxy.getCaCertificatePem()
        return ""
    }

    /**
     * Get current capture statistics.
     */
    fun getStats(): CaptureStats {
        // TODO: Call UniFFI-generated NetcapProxy.getStats()
        return CaptureStats(0, 0, 0, 0)
    }

    /**
     * Get captured events as JSON string.
     */
    fun getCaptureEvents(offset: Long = 0, limit: Long = 100): String {
        // TODO: Call UniFFI-generated NetcapProxy.getCaptureEvents()
        return "[]"
    }

    fun isProxyRunning(): Boolean = isRunning
}

/**
 * Data class representing capture statistics.
 */
data class CaptureStats(
    val totalRequests: Long,
    val totalResponses: Long,
    val activeConnections: Int,
    val bytesCaptured: Long
)
