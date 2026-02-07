// SPDX-License-Identifier: MIT
package com.buckets.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.buckets.app.ui.navigation.BucketsNavHost
import com.buckets.app.ui.theme.BucketsTheme
import dagger.hilt.android.AndroidEntryPoint

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            BucketsTheme {
                BucketsNavHost()
            }
        }
    }
}
