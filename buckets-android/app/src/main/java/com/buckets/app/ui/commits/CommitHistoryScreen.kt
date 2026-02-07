// SPDX-License-Identifier: MIT
package com.buckets.app.ui.commits

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
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
import com.buckets.app.data.api.CommitRow
import com.buckets.app.data.repository.BucketsRepository
import com.buckets.app.ui.components.ErrorState
import com.buckets.app.ui.components.LoadingIndicator
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

sealed interface CommitHistoryUiState {
    data object Loading : CommitHistoryUiState
    data class Success(val commits: List<CommitRow>) : CommitHistoryUiState
    data class Error(val message: String) : CommitHistoryUiState
}

@HiltViewModel
class CommitHistoryViewModel @Inject constructor(
    savedStateHandle: SavedStateHandle,
    private val repository: BucketsRepository,
) : ViewModel() {

    private val bucketId: String = savedStateHandle["bucketId"] ?: ""

    private val _uiState = MutableStateFlow<CommitHistoryUiState>(CommitHistoryUiState.Loading)
    val uiState: StateFlow<CommitHistoryUiState> = _uiState.asStateFlow()

    init {
        loadCommits()
    }

    fun loadCommits() {
        viewModelScope.launch {
            _uiState.value = CommitHistoryUiState.Loading
            repository.getBucketCommits(bucketId)
                .onSuccess { commits ->
                    _uiState.value = CommitHistoryUiState.Success(commits)
                }
                .onFailure { error ->
                    _uiState.value = CommitHistoryUiState.Error(
                        error.message ?: "Failed to load commits"
                    )
                }
        }
    }
}

@Composable
fun CommitHistoryScreen(
    bucketId: String,
    onCommitClick: (String) -> Unit,
    onNavigateBack: () -> Unit,
    viewModel: CommitHistoryViewModel = hiltViewModel(),
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
                text = "Commit History",
                style = MaterialTheme.typography.titleLarge,
            )
        }

        when (val state = uiState) {
            is CommitHistoryUiState.Loading -> LoadingIndicator()
            is CommitHistoryUiState.Error -> ErrorState(
                message = state.message,
                onRetry = { viewModel.loadCommits() },
            )
            is CommitHistoryUiState.Success -> {
                if (state.commits.isEmpty()) {
                    Text(
                        text = "No commits found",
                        style = MaterialTheme.typography.bodyLarge,
                        modifier = Modifier.padding(16.dp),
                    )
                } else {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize(),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        items(state.commits, key = { it.id }) { commit ->
                            CommitHistoryCard(
                                commit = commit,
                                onClick = { onCommitClick(commit.id) },
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun CommitHistoryCard(
    commit: CommitRow,
    onClick: () -> Unit,
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp)
            .clickable(onClick = onClick),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = commit.message,
                style = MaterialTheme.typography.bodyLarge,
            )
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 8.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    text = commit.bucketName,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
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
