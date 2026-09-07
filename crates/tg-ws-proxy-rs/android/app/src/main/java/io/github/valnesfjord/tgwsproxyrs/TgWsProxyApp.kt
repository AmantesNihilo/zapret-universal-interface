package io.github.valnesfjord.tgwsproxyrs

import android.app.Application

class TgWsProxyApp : Application() {
    override fun onCreate() {
        super.onCreate()
        NativeProxy.load()
        ProxyBridge.syncFromNative()
    }
}
