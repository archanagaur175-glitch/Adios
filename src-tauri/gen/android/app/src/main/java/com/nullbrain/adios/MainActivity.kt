package com.nullbrain.adios

import android.os.Bundle
import android.view.View
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    // Gate tooling: release APKs are non-debuggable so WebView hides its
    // devtools socket; the device gates (P19 19.5+) attach CDP to invoke
    // plugin commands and read logcat. Personal-use app — always on.
    WebView.setWebContentsDebuggingEnabled(true)
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // Bars stay visible (no fullscreen, nav buttons kept); pad the content
    // root so the status bar never covers titlebar and nav bar never
    // covers the player bar. Works on both opt-in (A12) and enforced (A15+)
    // edge-to-edge: systemBars insets are dispatched either way.
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { v, insets ->
      val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
      v.setPadding(bars.left, bars.top, bars.right, bars.bottom)
      insets
    }
    content.requestApplyInsets()
  }
}
