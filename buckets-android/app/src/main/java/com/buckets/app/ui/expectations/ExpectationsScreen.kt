// SPDX-License-Identifier: MIT
package com.buckets.app.ui.expectations

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import com.buckets.app.data.api.ExpectationRow
import com.buckets.app.data.api.SemanticResult
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

sealed interface ExpectationsUiState {
    data object Loading : ExpectationsUiState
    data class Success(
        val expectations: List<ExpectationRow>,
        val searchResults: List<SemanticResult>? = null,
    ) : ExpectationsUiState
    data class Error(val message: String) : ExpectationsUiState
}

@HiltViewModel
class ExpectationsViewModel @Inject constructor(
    private val repository: BucketsRepository,
) : ViewModel() {

    private val _uiState = MutableStateFlow<ExpectationsUiState>(ExpectationsUiState.Loading)
    val uiState: StateFlow<ExpectationsUiState> = _uiState.asStateFlow()

    init {
        loadExpectations()
    }

    fun loadExpectations() {
        viewModelScope.launch {
            _uiState.value = ExpectationsUiState.Loading
            repository.getExpectations()
                .onSuccess { expectations ->
                    _uiState.value = ExpectationsUiState.Success(expectations)
                }
                .onFailure { error ->
                    _uiState.value = ExpectationsUiState.Error(
                        error.message ?: "Failed to load expectations"
                    )
                }
        }
    }

    fun search(query: String) {
        if (query.isBlank()) {
            // Reset to showing all expectations
            loadExpectations()
            return
        }

        viewModelScope.launch {
            val currentState = _uiState.value
            val currentExpectations = if (currentState is ExpectationsUiState.Success) {
                currentState.expectations
            } else {
                emptyList()
            }

            repository.search(query)
                .onSuccess { results ->
                    _uiState.value = ExpectationsUiState.Success(
                        expectations = currentExpectations,
                        searchResults = results,
                    )
                }
                .onFailure { error ->
                    _uiState.value = ExpectationsUiState.Error(
                        error.message ?: "Search failed"
                    )
                }
        }
    }
}

@Composable
fun ExpectationsScreen(
    viewModel: ExpectationsViewModel = hiltViewModel(),
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    var searchQuery by rememberSaveable { mutableStateOf("") }

    Column(modifier = Modifier.fillMaxSize()) {
        TextField(
            value = searchQuery,
            onValueChange = { searchQuery = it },
            placeholder = { Text("Semantic search expectations...") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(
                onSearch = { viewModel.search(searchQuery) },
            ),
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
        )

        when (val state = uiState) {
            is ExpectationsUiState.Loading -> LoadingIndicator()
            is ExpectationsUiState.Error -> ErrorState(
                message = state.message,
                onRetry = { viewModel.loadExpectations() },
            )
            is ExpectationsUiState.Success -> {
                if (state.searchResults != null) {
                    SearchResultsList(results = state.searchResults)
                } else if (state.expectations.isEmpty()) {
                    Text(
                        text = "No expectations found",
                        style = MaterialTheme.typography.bodyLarge,
                        modifier = Modifier.padding(16.dp),
                    )
                } else {
                    ExpectationsList(expectations = state.expectations)
                }
            }
        }
    }
}

@Composable
private fun ExpectationsList(expectations: List<ExpectationRow>) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        items(expectations, key = { it.id }) { expectation ->
            ExpectationCard(expectation = expectation)
        }
    }
}

@Composable
private fun SearchResultsList(results: List<SemanticResult>) {
    if (results.isEmpty()) {
        Text(
            text = "No matching expectations found",
            style = MaterialTheme.typography.bodyLarge,
            modifier = Modifier.padding(16.dp),
        )
        return
    }

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        items(results, key = { it.expectation.id }) { result ->
            ExpectationCard(
                expectation = result.expectation,
                similarity = result.similarity,
            )
        }
    }
}

@Composable
private fun ExpectationCard(
    expectation: ExpectationRow,
    similarity: Double? = null,
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = expectation.description,
                    style = MaterialTheme.typography.bodyLarge,
                    modifier = Modifier.weight(1f),
                )
                StatusChip(status = expectation.status)
            }
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 8.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    text = "${expectation.sourceBucket} -> ${expectation.targetBucket ?: "N/A"}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    text = expectation.createdAt,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            if (similarity != null) {
                Text(
                    text = "Similarity: ${"%.1f".format(similarity * 100)}%",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }
        }
    }
}
