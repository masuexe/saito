package saito.nearby

import android.annotation.TargetApi
import android.app.Activity
import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import android.util.Log
import java.net.Inet4Address
import java.net.InetAddress
import java.util.concurrent.Executor

/**
 * saito-nearby's discovery and reverse-offer advertising on Android (crates/saito-nearby/src/platform/android.rs):
 * NsdManager browses for devices that offer (isolating API 34+ ServiceInfoCallback in [Api34]), resolves each to all
 * its IPv4 host addresses, port, and TXT `n`, and also registers `_saito-ask._tcp` while seeking so desktop offerers
 * can connect outbound when Windows firewall blocks inbound TCP. Acquires a Wi-Fi MulticastLock while active.
 */
object SaitoNearby {
    private const val TAG = "saito-nearby"

    private var manager: NsdManager? = null
    private var discovery: NsdManager.DiscoveryListener? = null
    private var registration: NsdManager.RegistrationListener? = null
    private var multicastLock: WifiManager.MulticastLock? = null

    /** The services found and not resolved yet (API < 34), retry tracking, and active API 34+ callbacks. */
    private val waiting = ArrayDeque<NsdServiceInfo>()
    private val retried = HashSet<String>()
    private val callbacks = HashMap<String, Any>()
    private var resolving = false
    private val mainHandler = Handler(Looper.getMainLooper())

    /** Browse for [service] (`_saito._tcp`), ending any browse already under way. */
    @JvmStatic
    fun start(activity: Activity, service: String) {
        synchronized(this) {
            stopLocked()
            val manager = activity.getSystemService(Context.NSD_SERVICE) as NsdManager
            val listener = object : NsdManager.DiscoveryListener {
                override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                    val current = synchronized(this@SaitoNearby) {
                        val isCurrent = discovery === this
                        if (isCurrent) {
                            discovery = null
                            updateMulticastLock(activity)
                        }
                        isCurrent
                    }
                    if (current) failed("NsdManager didn't start browsing (error $errorCode)")
                }

                override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
                    Log.w(TAG, "stopping the browse failed (error $errorCode)")
                }

                override fun onDiscoveryStarted(serviceType: String) {}

                override fun onDiscoveryStopped(serviceType: String) {}

                override fun onServiceFound(info: NsdServiceInfo) {
                    synchronized(this@SaitoNearby) {
                        if (discovery !== this) return
                        if (Build.VERSION.SDK_INT >= 34) {
                            val mgr = this@SaitoNearby.manager ?: return
                            if (!callbacks.containsKey(info.serviceName)) {
                                val cb = Api34.register(mgr, info, { updated ->
                                    val hosts = updated.hostAddresses
                                        .filterIsInstance<Inet4Address>()
                                        .mapNotNull { it.hostAddress }
                                        .distinct()
                                    val name = updated.attributes["n"]?.toString(Charsets.UTF_8) ?: ""
                                    val current = synchronized(this@SaitoNearby) { discovery === this }
                                    if (current && hosts.isNotEmpty()) {
                                        found(updated.serviceName, name, hosts.joinToString("\n"), updated.port)
                                    }
                                }, {
                                    val current = synchronized(this@SaitoNearby) {
                                        callbacks.remove(info.serviceName)
                                        discovery === this
                                    }
                                    if (current) lost(info.serviceName)
                                }, { errorCode ->
                                    Log.i(TAG, "resolving ${info.serviceName} failed (error $errorCode)")
                                    synchronized(this@SaitoNearby) { callbacks.remove(info.serviceName) }
                                })
                                if (cb != null) callbacks[info.serviceName] = cb
                            }
                        } else {
                            waiting.addLast(info)
                            resolveNext()
                        }
                    }
                }

                override fun onServiceLost(info: NsdServiceInfo) {
                    val removedCb = synchronized(this@SaitoNearby) {
                        if (discovery !== this) return
                        waiting.removeAll { it.serviceName == info.serviceName }
                        retried.remove(info.serviceName)
                        callbacks.remove(info.serviceName)
                    }
                    if (removedCb != null && Build.VERSION.SDK_INT >= 34) {
                        Api34.unregister(manager, removedCb)
                    }
                    lost(info.serviceName)
                }
            }
            this.manager = manager
            discovery = listener
            updateMulticastLock(activity)
            manager.discoverServices(service, NsdManager.PROTOCOL_DNS_SD, listener)
        }
    }

    /** End the browse. */
    @JvmStatic
    fun stop() {
        synchronized(this) { stopLocked() }
    }

    private fun stopLocked() {
        val listener = discovery
        discovery = null
        waiting.clear()
        retried.clear()
        val mgr = manager
        if (Build.VERSION.SDK_INT >= 34 && mgr != null) {
            for (cb in callbacks.values) {
                Api34.unregister(mgr, cb)
            }
        }
        callbacks.clear()
        updateMulticastLock(null)
        if (listener != null) {
            try {
                mgr?.stopServiceDiscovery(listener)
            } catch (e: IllegalArgumentException) {
                // (Never started: its start failed.)
            }
        }
    }

    /** Advertise [service] (e.g. `_saito-ask._tcp`) with device [name] on [port], ending any prior advertisement. */
    @JvmStatic
    fun advertise(activity: Activity, service: String, name: String, port: Int) {
        synchronized(this) {
            stopAdvertisingLocked()
            val mgr = manager ?: (activity.getSystemService(Context.NSD_SERVICE) as NsdManager).also { manager = it }
            val instance = truncateUtf8(name.replace(".", ""), 50).ifBlank { "saito" }
            val txtName = truncateUtf8(name, 253)
            val info = NsdServiceInfo().apply {
                serviceName = instance
                serviceType = service
                setPort(port)
                setAttribute("v", "1")
                setAttribute("n", txtName)
            }
            val listener = object : NsdManager.RegistrationListener {
                override fun onServiceRegistered(serviceInfo: NsdServiceInfo) {}

                override fun onRegistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                    Log.w(TAG, "advertising $service failed (error $errorCode)")
                    synchronized(this@SaitoNearby) {
                        if (registration === this) {
                            registration = null
                            updateMulticastLock(activity)
                        }
                    }
                }

                override fun onServiceUnregistered(serviceInfo: NsdServiceInfo) {}

                override fun onUnregistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                    Log.w(TAG, "stopping advertisement failed (error $errorCode)")
                }
            }
            registration = listener
            updateMulticastLock(activity)
            mgr.registerService(info, NsdManager.PROTOCOL_DNS_SD, listener)
        }
    }

    /** End advertising. */
    @JvmStatic
    fun stopAdvertising() {
        synchronized(this) { stopAdvertisingLocked() }
    }

    private fun stopAdvertisingLocked() {
        val listener = registration ?: return
        registration = null
        updateMulticastLock(null)
        try {
            manager?.unregisterService(listener)
        } catch (e: IllegalArgumentException) {
            // (Never registered: its start failed.)
        }
    }

    private fun updateMulticastLock(activity: Activity?) {
        val needed = discovery != null || registration != null
        try {
            if (needed) {
                if (multicastLock == null && activity != null) {
                    val wifi = activity.applicationContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager
                    multicastLock = wifi?.createMulticastLock("saito-nearby")?.apply {
                        setReferenceCounted(false)
                    }
                }
                val lock = multicastLock
                if (lock != null && !lock.isHeld) {
                    lock.acquire()
                }
            } else {
                val lock = multicastLock
                if (lock != null && lock.isHeld) {
                    lock.release()
                }
            }
        } catch (t: Throwable) {
            Log.w(TAG, "multicast lock: ${t.message}")
        }
    }

    /** Resolve the next service waiting, unless one is being resolved (with the lock held, API < 34). */
    @Suppress("DEPRECATION")
    private fun resolveNext() {
        if (resolving) return
        val info = waiting.removeFirstOrNull() ?: return
        val mgr = manager ?: return
        val listener = discovery
        resolving = true
        mgr.resolveService(info, object : NsdManager.ResolveListener {
            override fun onResolveFailed(failedInfo: NsdServiceInfo, errorCode: Int) {
                Log.i(TAG, "resolving ${failedInfo.serviceName} failed (error $errorCode)")
                val shouldRetry = errorCode == NsdManager.FAILURE_ALREADY_ACTIVE && synchronized(this@SaitoNearby) {
                    discovery === listener && retried.add(failedInfo.serviceName)
                }
                done()
                if (shouldRetry) {
                    mainHandler.postDelayed({
                        synchronized(this@SaitoNearby) {
                            if (discovery === listener) {
                                waiting.addLast(failedInfo)
                                resolveNext()
                            }
                        }
                    }, 150)
                }
            }

            override fun onServiceResolved(resolved: NsdServiceInfo) {
                val hosts = addresses(resolved)
                    .filterIsInstance<Inet4Address>()
                    .mapNotNull { it.hostAddress }
                    .distinct()
                val name = resolved.attributes["n"]?.toString(Charsets.UTF_8) ?: ""
                val current = synchronized(this@SaitoNearby) {
                    retried.remove(resolved.serviceName)
                    discovery === listener
                }
                if (current && hosts.isNotEmpty()) {
                    found(resolved.serviceName, name, hosts.joinToString("\n"), resolved.port)
                }
                done()
            }

            fun done() {
                synchronized(this@SaitoNearby) {
                    resolving = false
                    resolveNext()
                }
            }
        })
    }

    @Suppress("DEPRECATION")
    private fun addresses(info: NsdServiceInfo): List<InetAddress> =
        if (Build.VERSION.SDK_INT >= 34) info.hostAddresses else listOfNotNull(info.host)

    private fun truncateUtf8(s: String, maxBytes: Int): String {
        val trimmed = s.trim()
        var bytes = 0
        var end = 0
        var i = 0
        while (i < trimmed.length) {
            val cp = trimmed.codePointAt(i)
            val charLen = Character.charCount(cp)
            val utf8Len = when {
                cp <= 0x7F -> 1
                cp <= 0x7FF -> 2
                cp <= 0xFFFF -> 3
                else -> 4
            }
            if (bytes + utf8Len > maxBytes) break
            bytes += utf8Len
            i += charLen
            end = i
        }
        return trimmed.substring(0, end)
    }

    /** Isolated API 34+ ServiceInfoCallback wrapper so ART on Android 8-13 never verifies missing classes. */
    @TargetApi(34)
    private object Api34 {
        private val directExecutor = Executor { it.run() }

        fun register(
            manager: NsdManager,
            info: NsdServiceInfo,
            onUpdated: (NsdServiceInfo) -> Unit,
            onLost: () -> Unit,
            onFailed: (Int) -> Unit
        ): NsdManager.ServiceInfoCallback? {
            val cb = object : NsdManager.ServiceInfoCallback {
                override fun onServiceInfoCallbackRegistrationFailed(errorCode: Int) {
                    onFailed(errorCode)
                }

                override fun onServiceUpdated(serviceInfo: NsdServiceInfo) {
                    onUpdated(serviceInfo)
                }

                override fun onServiceLost() {
                    onLost()
                }

                override fun onServiceInfoCallbackUnregistered() {}
            }
            return try {
                manager.registerServiceInfoCallback(info, directExecutor, cb)
                cb
            } catch (t: Throwable) {
                Log.w(TAG, "registerServiceInfoCallback failed: ${t.message}")
                null
            }
        }

        fun unregister(manager: NsdManager, callback: Any) {
            val cb = callback as? NsdManager.ServiceInfoCallback ?: return
            try {
                manager.unregisterServiceInfoCallback(cb)
            } catch (t: Throwable) {
                // Already unregistered or registration failed.
            }
        }
    }

    /** The device's name as its player set it (Settings, About phone), else its model. */
    @JvmStatic
    fun deviceName(activity: Activity): String =
        Settings.Global.getString(activity.contentResolver, Settings.Global.DEVICE_NAME)?.takeIf { it.isNotBlank() } ?: Build.MODEL

    @JvmStatic
    private external fun found(key: String, name: String, host: String, port: Int)

    @JvmStatic
    private external fun lost(key: String)

    @JvmStatic
    private external fun failed(why: String)
}
