// SPDX-License-Identifier: MIT
package com.buckets.app.ui.navigation

sealed class Screen(val route: String) {
    data object BucketsList : Screen("buckets_list")
    data class BucketDetail(val id: String) : Screen("bucket_detail/$id") {
        companion object {
            const val ROUTE = "bucket_detail/{id}"
            fun createRoute(id: String) = "bucket_detail/$id"
        }
    }

    data class CommitHistory(val bucketId: String) : Screen("commit_history/$bucketId") {
        companion object {
            const val ROUTE = "commit_history/{bucketId}"
            fun createRoute(bucketId: String) = "commit_history/$bucketId"
        }
    }

    data class CommitDetail(val commitId: String) : Screen("commit_detail/$commitId") {
        companion object {
            const val ROUTE = "commit_detail/{commitId}"
            fun createRoute(commitId: String) = "commit_detail/$commitId"
        }
    }

    data object ExpectationsList : Screen("expectations_list")
    data object PebblesList : Screen("pebbles_list")
    data object Graph : Screen("graph")
    data object Settings : Screen("settings")
}
