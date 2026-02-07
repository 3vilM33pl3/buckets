// SPDX-License-Identifier: MIT
package com.buckets.app.data.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class BucketRow(
    val id: String,
    val name: String,
    val path: String,
    @SerialName("created_at") val createdAt: String,
    @SerialName("commit_count") val commitCount: Int,
    @SerialName("expectation_count") val expectationCount: Int,
)

@Serializable
data class CommitRow(
    val id: String,
    @SerialName("bucket_name") val bucketName: String,
    val message: String,
    @SerialName("file_count") val fileCount: Int,
    @SerialName("created_at") val createdAt: String,
)

@Serializable
data class FileRow(
    val id: String,
    @SerialName("file_path") val filePath: String,
    val hash: String,
)

@Serializable
data class ExpectationRow(
    val id: String,
    val description: String,
    val status: String,
    @SerialName("source_bucket") val sourceBucket: String,
    @SerialName("target_bucket") val targetBucket: String? = null,
    @SerialName("created_at") val createdAt: String,
)

@Serializable
data class PebbleRow(
    val id: String,
    val description: String,
    @SerialName("origin_bucket") val originBucket: String,
    @SerialName("current_bucket") val currentBucket: String,
    val status: String,
    @SerialName("created_at") val createdAt: String,
)

@Serializable
data class GraphData(
    val nodes: List<GraphNode>,
    val edges: List<GraphEdge>,
)

@Serializable
data class GraphNode(
    val id: String,
    val name: String,
)

@Serializable
data class GraphEdge(
    val from: String,
    val to: String,
    val count: Int,
)

@Serializable
data class SemanticResult(
    val expectation: ExpectationRow,
    val similarity: Double,
)

@Serializable
data class HealthResponse(
    val status: String,
)
