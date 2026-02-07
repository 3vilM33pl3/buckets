// SPDX-License-Identifier: MIT
package com.buckets.app.ui.components

import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.buckets.app.ui.theme.BucketGray
import com.buckets.app.ui.theme.BucketGrayLight
import com.buckets.app.ui.theme.BucketGreen
import com.buckets.app.ui.theme.BucketGreenLight
import com.buckets.app.ui.theme.BucketOrange
import com.buckets.app.ui.theme.BucketOrangeLight
import com.buckets.app.ui.theme.BucketRed
import com.buckets.app.ui.theme.BucketRedLight

@Composable
fun StatusChip(
    status: String,
    modifier: Modifier = Modifier,
) {
    val (backgroundColor, textColor) = statusColors(status)

    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(12.dp),
        color = backgroundColor,
    ) {
        Text(
            text = status,
            style = MaterialTheme.typography.labelMedium,
            color = textColor,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
        )
    }
}

private fun statusColors(status: String): Pair<Color, Color> {
    return when (status.lowercase()) {
        "pending" -> BucketOrangeLight to BucketOrange
        "fulfilled", "complete", "completed" -> BucketGreenLight to BucketGreen
        "rejected", "failed" -> BucketRedLight to BucketRed
        else -> BucketGrayLight to BucketGray
    }
}
