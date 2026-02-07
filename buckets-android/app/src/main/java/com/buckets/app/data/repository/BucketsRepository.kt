// SPDX-License-Identifier: MIT
package com.buckets.app.data.repository

import com.buckets.app.data.api.BucketRow
import com.buckets.app.data.api.BucketsApi
import com.buckets.app.data.api.CommitRow
import com.buckets.app.data.api.ExpectationRow
import com.buckets.app.data.api.FileRow
import com.buckets.app.data.api.GraphData
import com.buckets.app.data.api.HealthResponse
import com.buckets.app.data.api.PebbleRow
import com.buckets.app.data.api.SemanticResult
import javax.inject.Inject

class BucketsRepository @Inject constructor(
    private val api: BucketsApi,
) {
    suspend fun health(): Result<HealthResponse> = runCatching {
        api.health()
    }

    suspend fun getBuckets(): Result<List<BucketRow>> = runCatching {
        api.getBuckets()
    }

    suspend fun getBucket(id: String): Result<BucketRow> = runCatching {
        api.getBucket(id)
    }

    suspend fun getBucketCommits(id: String): Result<List<CommitRow>> = runCatching {
        api.getBucketCommits(id)
    }

    suspend fun getBucketExpectations(id: String): Result<List<ExpectationRow>> = runCatching {
        api.getBucketExpectations(id)
    }

    suspend fun getBucketPebbles(id: String): Result<List<PebbleRow>> = runCatching {
        api.getBucketPebbles(id)
    }

    suspend fun getCommitFiles(id: String): Result<List<FileRow>> = runCatching {
        api.getCommitFiles(id)
    }

    suspend fun getExpectations(): Result<List<ExpectationRow>> = runCatching {
        api.getExpectations()
    }

    suspend fun getPebbles(): Result<List<PebbleRow>> = runCatching {
        api.getPebbles()
    }

    suspend fun getGraph(): Result<GraphData> = runCatching {
        api.getGraph()
    }

    suspend fun search(query: String): Result<List<SemanticResult>> = runCatching {
        api.search(query)
    }
}
