// SPDX-License-Identifier: MIT
package com.buckets.app.ui.buckets

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import com.buckets.app.data.api.BucketRow
import com.buckets.app.data.repository.BucketsRepository
import com.buckets.app.ui.components.ErrorState
import com.buckets.app.ui.components.LoadingIndicator
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

sealed interface BucketsListUiState {
    data object Loading : BucketsListUiState
    data class Success(val buckets: List<BucketRow>) : BucketsListUiState
    data class Error(val message: String) : BucketsListUiState
}

@HiltViewModel
class BucketsListViewModel @Inject constructor(
    private val repository: BucketsRepository,
) : ViewModel() {

    private val _uiState = MutableStateFlow<BucketsListUiState>(BucketsListUiState.Loading)
    val uiState: StateFlow<BucketsListUiState> = _uiState.asStateFlow()

    private val _isRefreshing = MutableStateFlow(false)
    val isRefreshing: StateFlow<Boolean> = _isRefreshing.asStateFlow()

    private var allBuckets: List<BucketRow> = emptyList()

    init {
        loadBuckets()
    }

    fun loadBuckets() {
        viewModelScope.launch {
            _uiState.value = BucketsListUiState.Loading
            repository.getBuckets()
                .onSuccess { buckets ->
                    allBuckets = buckets
                    _uiState.value = BucketsListUiState.Success(buckets)
                }
                .onFailure { error ->
                    _uiState.value = BucketsListUiState.Error(
                        error.message ?: "Failed to load buckets"
                    )
                }
        }
    }

    fun refresh() {
        viewModelScope.launch {
            _isRefreshing.value = true
            repository.getBuckets()
                .onSuccess { buckets ->
                    allBuckets = buckets
                    _uiState.value = BucketsListUiState.Success(buckets)
                }
                .onFailure { error ->
                    _uiState.value = BucketsListUiState.Error(
                        error.message ?: "Failed to load buckets"
                    )
                }
            _isRefreshing.value = false
        }
    }

    fun filter(query: String) {
        if (query.isBlank()) {
            _uiState.value = BucketsListUiState.Success(allBuckets)
        } else {
            val filtered = allBuckets.filter {
                it.name.contains(query, ignoreCase = true) ||
                    it.path.contains(query, ignoreCase = true)
            }
            _uiState.value = BucketsListUiState.Success(filtered)
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BucketsListScreen(
    onBucketClick: (String) -> Unit,
    viewModel: BucketsListViewModel = hiltViewModel(),
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val isRefreshing by viewModel.isRefreshing.collectAsStateWithLifecycle()
    var searchQuery by rememberSaveable { mutableStateOf("") }

    Column(modifier = Modifier.fillMaxSize()) {
        TextField(
            value = searchQuery,
            onValueChange = { query ->
                searchQuery = query
                viewModel.filter(query)
            },
            placeholder = { Text("Search buckets...") },
            singleLine = true,
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
        )

        when (val state = uiState) {
            is BucketsListUiState.Loading -> LoadingIndicator()
            is BucketsListUiState.Error -> ErrorState(
                message = state.message,
                onRetry = { viewModel.loadBuckets() },
            )
            is BucketsListUiState.Success -> {
                PullToRefreshBox(
                    isRefreshing = isRefreshing,
                    onRefresh = { viewModel.refresh() },
                    modifier = Modifier.fillMaxSize(),
                ) {
                    if (state.buckets.isEmpty()) {
                        Text(
                            text = "No buckets found",
                            style = MaterialTheme.typography.bodyLarge,
                            modifier = Modifier.padding(16.dp),
                        )
                    } else {
                        LazyColumn(
                            modifier = Modifier.fillMaxSize(),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            items(state.buckets, key = { it.id }) { bucket ->
                                BucketCard(
                                    bucket = bucket,
                                    onClick = { onBucketClick(bucket.id) },
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun BucketCard(
    bucket: BucketRow,
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
                text = bucket.name,
                style = MaterialTheme.typography.titleMedium,
            )
            Text(
                text = bucket.path,
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
                    text = "${bucket.commitCount} commits",
                    style = MaterialTheme.typography.labelMedium,
                )
                Text(
                    text = "${bucket.expectationCount} expectations",
                    style = MaterialTheme.typography.labelMedium,
                )
            }
        }
    }
}
