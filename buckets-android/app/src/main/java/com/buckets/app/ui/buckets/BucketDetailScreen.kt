// SPDX-License-Identifier: MIT
package com.buckets.app.ui.buckets

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import com.buckets.app.data.api.BucketRow
import com.buckets.app.data.api.CommitRow
import com.buckets.app.data.api.ExpectationRow
import com.buckets.app.data.api.PebbleRow
import com.buckets.app.data.repository.BucketsRepository
import com.buckets.app.ui.components.ErrorState
import com.buckets.app.ui.components.LoadingIndicator
import com.buckets.app.ui.components.StatusChip
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

data class BucketDetailData(
    val bucket: BucketRow,
    val commits: List<CommitRow>,
    val expectations: List<ExpectationRow>,
    val pebbles: List<PebbleRow>,
)

sealed interface BucketDetailUiState {
    data object Loading : BucketDetailUiState
    data class Success(val data: BucketDetailData) : BucketDetailUiState
    data class Error(val message: String) : BucketDetailUiState
}

@HiltViewModel
class BucketDetailViewModel @Inject constructor(
    savedStateHandle: SavedStateHandle,
    private val repository: BucketsRepository,
) : ViewModel() {

    private val bucketId: String = savedStateHandle["id"] ?: ""

    private val _uiState = MutableStateFlow<BucketDetailUiState>(BucketDetailUiState.Loading)
    val uiState: StateFlow<BucketDetailUiState> = _uiState.asStateFlow()

    init {
        loadBucketDetail()
    }

    fun loadBucketDetail() {
        viewModelScope.launch {
            _uiState.value = BucketDetailUiState.Loading

            val bucketResult = repository.getBucket(bucketId)
            val commitsResult = repository.getBucketCommits(bucketId)
            val expectationsResult = repository.getBucketExpectations(bucketId)
            val pebblesResult = repository.getBucketPebbles(bucketId)

            val bucket = bucketResult.getOrNull()
            if (bucket == null) {
                _uiState.value = BucketDetailUiState.Error(
                    bucketResult.exceptionOrNull()?.message ?: "Failed to load bucket"
                )
                return@launch
            }

            _uiState.value = BucketDetailUiState.Success(
                BucketDetailData(
                    bucket = bucket,
                    commits = commitsResult.getOrDefault(emptyList()),
                    expectations = expectationsResult.getOrDefault(emptyList()),
                    pebbles = pebblesResult.getOrDefault(emptyList()),
                )
            )
        }
    }
}

@Composable
fun BucketDetailScreen(
    bucketId: String,
    onCommitClick: (String) -> Unit,
    onViewAllCommits: () -> Unit,
    onNavigateBack: () -> Unit,
    viewModel: BucketDetailViewModel = hiltViewModel(),
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(horizontal = 4.dp, vertical = 4.dp),
        ) {
            IconButton(onClick = onNavigateBack) {
                Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
            }
            Text(
                text = "Bucket Detail",
                style = MaterialTheme.typography.titleLarge,
            )
        }

        when (val state = uiState) {
            is BucketDetailUiState.Loading -> LoadingIndicator()
            is BucketDetailUiState.Error -> ErrorState(
                message = state.message,
                onRetry = { viewModel.loadBucketDetail() },
            )
            is BucketDetailUiState.Success -> {
                BucketDetailContent(
                    data = state.data,
                    onCommitClick = onCommitClick,
                    onViewAllCommits = onViewAllCommits,
                )
            }
        }
    }
}

@Composable
private fun BucketDetailContent(
    data: BucketDetailData,
    onCommitClick: (String) -> Unit,
    onViewAllCommits: () -> Unit,
) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        // Info section
        item {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp),
                elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text(
                        text = data.bucket.name,
                        style = MaterialTheme.typography.headlineMedium,
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = "Path: ${data.bucket.path}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        text = "Created: ${data.bucket.createdAt}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(top = 8.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(
                            text = "${data.bucket.commitCount} commits",
                            style = MaterialTheme.typography.labelMedium,
                        )
                        Text(
                            text = "${data.bucket.expectationCount} expectations",
                            style = MaterialTheme.typography.labelMedium,
                        )
                    }
                }
            }
        }

        // Commits section
        item {
            SectionHeader(
                title = "Commits (${data.commits.size})",
                onViewAll = if (data.commits.size > 3) onViewAllCommits else null,
            )
        }
        items(data.commits.take(3), key = { "commit-${it.id}" }) { commit ->
            CommitCard(commit = commit, onClick = { onCommitClick(commit.id) })
        }

        // Expectations section
        item {
            SectionHeader(title = "Expectations (${data.expectations.size})")
        }
        items(data.expectations, key = { "exp-${it.id}" }) { expectation ->
            ExpectationCard(expectation = expectation)
        }

        // Pebbles section
        item {
            SectionHeader(title = "Pebbles (${data.pebbles.size})")
        }
        items(data.pebbles, key = { "peb-${it.id}" }) { pebble ->
            PebbleCard(pebble = pebble)
        }

        item { Spacer(modifier = Modifier.height(16.dp)) }
    }
}

@Composable
private fun SectionHeader(
    title: String,
    onViewAll: (() -> Unit)? = null,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
        )
        if (onViewAll != null) {
            TextButton(onClick = onViewAll) {
                Text("View All")
            }
        }
    }
    HorizontalDivider(modifier = Modifier.padding(horizontal = 16.dp))
}

@Composable
private fun CommitCard(
    commit: CommitRow,
    onClick: () -> Unit,
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp)
            .clickable(onClick = onClick),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(
                text = commit.message,
                style = MaterialTheme.typography.bodyLarge,
            )
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 4.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    text = "${commit.fileCount} files",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    text = commit.createdAt,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun ExpectationCard(expectation: ExpectationRow) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = expectation.description,
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.weight(1f),
                )
                StatusChip(status = expectation.status)
            }
            Text(
                text = "${expectation.sourceBucket} -> ${expectation.targetBucket ?: "N/A"}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp),
            )
        }
    }
}

@Composable
private fun PebbleCard(pebble: PebbleRow) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = pebble.description,
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.weight(1f),
                )
                StatusChip(status = pebble.status)
            }
            Text(
                text = "${pebble.originBucket} -> ${pebble.currentBucket}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp),
            )
        }
    }
}
