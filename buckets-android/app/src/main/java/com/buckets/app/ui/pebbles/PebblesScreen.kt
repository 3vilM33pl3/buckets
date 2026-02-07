// SPDX-License-Identifier: MIT
package com.buckets.app.ui.pebbles

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
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
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

sealed interface PebblesUiState {
    data object Loading : PebblesUiState
    data class Success(val pebbles: List<PebbleRow>) : PebblesUiState
    data class Error(val message: String) : PebblesUiState
}

@HiltViewModel
class PebblesViewModel @Inject constructor(
    private val repository: BucketsRepository,
) : ViewModel() {

    private val _uiState = MutableStateFlow<PebblesUiState>(PebblesUiState.Loading)
    val uiState: StateFlow<PebblesUiState> = _uiState.asStateFlow()

    init {
        loadPebbles()
    }

    fun loadPebbles() {
        viewModelScope.launch {
            _uiState.value = PebblesUiState.Loading
            repository.getPebbles()
                .onSuccess { pebbles ->
                    _uiState.value = PebblesUiState.Success(pebbles)
                }
                .onFailure { error ->
                    _uiState.value = PebblesUiState.Error(
                        error.message ?: "Failed to load pebbles"
                    )
                }
        }
    }
}

@Composable
fun PebblesScreen(
    viewModel: PebblesViewModel = hiltViewModel(),
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    Column(modifier = Modifier.fillMaxSize()) {
        when (val state = uiState) {
            is PebblesUiState.Loading -> LoadingIndicator()
            is PebblesUiState.Error -> ErrorState(
                message = state.message,
                onRetry = { viewModel.loadPebbles() },
            )
            is PebblesUiState.Success -> {
                if (state.pebbles.isEmpty()) {
                    Text(
                        text = "No pebbles found",
                        style = MaterialTheme.typography.bodyLarge,
                        modifier = Modifier.padding(16.dp),
                    )
                } else {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize(),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        items(state.pebbles, key = { it.id }) { pebble ->
                            PebbleCard(pebble = pebble)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun PebbleCard(pebble: PebbleRow) {
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
                    text = pebble.description,
                    style = MaterialTheme.typography.bodyLarge,
                    modifier = Modifier.weight(1f),
                )
                StatusChip(status = pebble.status)
            }
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 8.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    text = "${pebble.originBucket} -> ${pebble.currentBucket}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    text = pebble.createdAt,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
