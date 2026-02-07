// SPDX-License-Identifier: MIT
package com.buckets.app.ui.commits

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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import com.buckets.app.data.api.FileRow
import com.buckets.app.data.repository.BucketsRepository
import com.buckets.app.ui.components.ErrorState
import com.buckets.app.ui.components.LoadingIndicator
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

sealed interface CommitDetailUiState {
    data object Loading : CommitDetailUiState
    data class Success(val files: List<FileRow>) : CommitDetailUiState
    data class Error(val message: String) : CommitDetailUiState
}

@HiltViewModel
class CommitDetailViewModel @Inject constructor(
    savedStateHandle: SavedStateHandle,
    private val repository: BucketsRepository,
) : ViewModel() {

    private val commitId: String = savedStateHandle["commitId"] ?: ""

    private val _uiState = MutableStateFlow<CommitDetailUiState>(CommitDetailUiState.Loading)
    val uiState: StateFlow<CommitDetailUiState> = _uiState.asStateFlow()

    init {
        loadFiles()
    }

    fun loadFiles() {
        viewModelScope.launch {
            _uiState.value = CommitDetailUiState.Loading
            repository.getCommitFiles(commitId)
                .onSuccess { files ->
                    _uiState.value = CommitDetailUiState.Success(files)
                }
                .onFailure { error ->
                    _uiState.value = CommitDetailUiState.Error(
                        error.message ?: "Failed to load commit files"
                    )
                }
        }
    }
}

@Composable
fun CommitDetailScreen(
    commitId: String,
    onNavigateBack: () -> Unit,
    viewModel: CommitDetailViewModel = hiltViewModel(),
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
            Column {
                Text(
                    text = "Commit Detail",
                    style = MaterialTheme.typography.titleLarge,
                )
                Text(
                    text = commitId,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }

        when (val state = uiState) {
            is CommitDetailUiState.Loading -> LoadingIndicator()
            is CommitDetailUiState.Error -> ErrorState(
                message = state.message,
                onRetry = { viewModel.loadFiles() },
            )
            is CommitDetailUiState.Success -> {
                if (state.files.isEmpty()) {
                    Text(
                        text = "No files in this commit",
                        style = MaterialTheme.typography.bodyLarge,
                        modifier = Modifier.padding(16.dp),
                    )
                } else {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize(),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        items(state.files, key = { it.id }) { file ->
                            FileCard(file = file)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun FileCard(file: FileRow) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(
                text = file.filePath,
                style = MaterialTheme.typography.bodyMedium,
            )
            Text(
                text = "Hash: ${file.hash}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(top = 4.dp),
            )
        }
    }
}
