// SPDX-License-Identifier: MIT
package com.buckets.app.data.api

import retrofit2.http.GET
import retrofit2.http.Path
import retrofit2.http.Query

interface BucketsApi {

    @GET("/api/health")
    suspend fun health(): HealthResponse

    @GET("/api/buckets")
    suspend fun getBuckets(): List<BucketRow>

    @GET("/api/buckets/{id}")
    suspend fun getBucket(@Path("id") id: String): BucketRow

    @GET("/api/buckets/{id}/commits")
    suspend fun getBucketCommits(@Path("id") id: String): List<CommitRow>

    @GET("/api/buckets/{id}/expectations")
    suspend fun getBucketExpectations(@Path("id") id: String): List<ExpectationRow>

    @GET("/api/buckets/{id}/pebbles")
    suspend fun getBucketPebbles(@Path("id") id: String): List<PebbleRow>

    @GET("/api/commits/{id}/files")
    suspend fun getCommitFiles(@Path("id") id: String): List<FileRow>

    @GET("/api/expectations")
    suspend fun getExpectations(): List<ExpectationRow>

    @GET("/api/pebbles")
    suspend fun getPebbles(): List<PebbleRow>

    @GET("/api/graph")
    suspend fun getGraph(): GraphData

    @GET("/api/search")
    suspend fun search(@Query("q") query: String): List<SemanticResult>
}
