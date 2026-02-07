// SPDX-License-Identifier: MIT
package com.buckets.app.ui.graph

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import com.buckets.app.data.api.GraphData
import com.buckets.app.data.api.GraphEdge
import com.buckets.app.data.api.GraphNode
import com.buckets.app.data.repository.BucketsRepository
import com.buckets.app.ui.components.ErrorState
import com.buckets.app.ui.components.LoadingIndicator
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

sealed interface GraphUiState {
    data object Loading : GraphUiState
    data class Success(val data: GraphData) : GraphUiState
    data class Error(val message: String) : GraphUiState
}

@HiltViewModel
class GraphViewModel @Inject constructor(
    private val repository: BucketsRepository,
) : ViewModel() {

    private val _uiState = MutableStateFlow<GraphUiState>(GraphUiState.Loading)
    val uiState: StateFlow<GraphUiState> = _uiState.asStateFlow()

    init {
        loadGraph()
    }

    fun loadGraph() {
        viewModelScope.launch {
            _uiState.value = GraphUiState.Loading
            repository.getGraph()
                .onSuccess { graphData ->
                    _uiState.value = GraphUiState.Success(graphData)
                }
                .onFailure { error ->
                    _uiState.value = GraphUiState.Error(
                        error.message ?: "Failed to load graph"
                    )
                }
        }
    }
}

@Composable
fun GraphScreen(
    viewModel: GraphViewModel = hiltViewModel(),
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    Column(modifier = Modifier.fillMaxSize()) {
        when (val state = uiState) {
            is GraphUiState.Loading -> LoadingIndicator()
            is GraphUiState.Error -> ErrorState(
                message = state.message,
                onRetry = { viewModel.loadGraph() },
            )
            is GraphUiState.Success -> {
                GraphContent(data = state.data)
            }
        }
    }
}

@Composable
private fun GraphContent(data: GraphData) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        // Nodes section
        item {
            Text(
                text = "Nodes (${data.nodes.size})",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
            )
            HorizontalDivider(modifier = Modifier.padding(horizontal = 16.dp))
        }
        items(data.nodes, key = { "node-${it.id}" }) { node ->
            NodeCard(node = node)
        }

        // Edges section
        item {
            Spacer(modifier = Modifier.height(16.dp))
            Text(
                text = "Edges (${data.edges.size})",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
            )
            HorizontalDivider(modifier = Modifier.padding(horizontal = 16.dp))
        }
        items(data.edges, key = { "edge-${it.from}-${it.to}" }) { edge ->
            EdgeCard(edge = edge, nodes = data.nodes)
        }

        item { Spacer(modifier = Modifier.height(16.dp)) }
    }
}

@Composable
private fun NodeCard(node: GraphNode) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(
                text = node.name,
                style = MaterialTheme.typography.bodyLarge,
            )
            Text(
                text = node.id,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun EdgeCard(
    edge: GraphEdge,
    nodes: List<GraphNode>,
) {
    val fromName = nodes.find { it.id == edge.from }?.name ?: edge.from
    val toName = nodes.find { it.id == edge.to }?.name ?: edge.to

    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(
                text = "$fromName -> $toName",
                style = MaterialTheme.typography.bodyLarge,
            )
            Text(
                text = "Count: ${edge.count}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
