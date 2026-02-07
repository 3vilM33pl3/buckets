// SPDX-License-Identifier: MIT
package com.buckets.app.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable

private val LightColorScheme = lightColorScheme(
    primary = BucketPrimary,
    primaryContainer = BucketPrimaryContainer,
    secondary = BucketSecondary,
    secondaryContainer = BucketSecondaryContainer,
    surface = BucketSurface,
    background = BucketBackground,
    onPrimary = BucketOnPrimary,
    onSurface = BucketOnSurface,
    onSurfaceVariant = BucketOnSurfaceVariant,
)

@Composable
fun BucketsTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = LightColorScheme,
        typography = BucketsTypography,
        content = content,
    )
}
