// SPDX-License-Identifier: MIT
package com.buckets.app.ui.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import com.buckets.app.data.repository.BucketsRepository
import com.buckets.app.ui.theme.BucketGreen
import com.buckets.app.ui.theme.BucketOrange
import com.buckets.app.ui.theme.BucketRed
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import javax.inject.Inject

sealed interface ConnectionStatus {
    data object Idle : ConnectionStatus
    data object Testing : ConnectionStatus
    data class Connected(val status: String) : ConnectionStatus
    data class Failed(val message: String) : ConnectionStatus
}

@HiltViewModel
class SettingsViewModel @Inject constructor(
    private val dataStore: DataStore<Preferences>,
    private val repository: BucketsRepository,
) : ViewModel() {

    companion object {
        val SERVER_URL_KEY = stringPreferencesKey("server_url")
        const val DEFAULT_SERVER_URL = "http://10.0.2.2:3000"
    }

    private val _serverUrl = MutableStateFlow(DEFAULT_SERVER_URL)
    val serverUrl: StateFlow<String> = _serverUrl.asStateFlow()

    private val _connectionStatus = MutableStateFlow<ConnectionStatus>(ConnectionStatus.Idle)
    val connectionStatus: StateFlow<ConnectionStatus> = _connectionStatus.asStateFlow()

    init {
        viewModelScope.launch {
            val savedUrl = dataStore.data
                .map { preferences -> preferences[SERVER_URL_KEY] ?: DEFAULT_SERVER_URL }
                .first()
            _serverUrl.value = savedUrl
        }
    }

    fun updateServerUrl(url: String) {
        _serverUrl.value = url
    }

    fun saveServerUrl() {
        viewModelScope.launch {
            dataStore.edit { preferences ->
                preferences[SERVER_URL_KEY] = _serverUrl.value
            }
        }
    }

    fun testConnection() {
        viewModelScope.launch {
            _connectionStatus.value = ConnectionStatus.Testing
            repository.health()
                .onSuccess { response ->
                    _connectionStatus.value = ConnectionStatus.Connected(response.status)
                }
                .onFailure { error ->
                    _connectionStatus.value = ConnectionStatus.Failed(
                        error.message ?: "Connection failed"
                    )
                }
        }
    }
}

@Composable
fun SettingsScreen(
    onNavigateBack: () -> Unit,
    viewModel: SettingsViewModel = hiltViewModel(),
) {
    val serverUrl by viewModel.serverUrl.collectAsStateWithLifecycle()
    val connectionStatus by viewModel.connectionStatus.collectAsStateWithLifecycle()

    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(horizontal = 4.dp, vertical = 4.dp),
        ) {
            IconButton(onClick = onNavigateBack) {
                Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
            }
            Text(
                text = "Settings",
                style = MaterialTheme.typography.titleLarge,
            )
        }

        Card(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "Server Configuration",
                    style = MaterialTheme.typography.titleMedium,
                )

                Spacer(modifier = Modifier.height(12.dp))

                OutlinedTextField(
                    value = serverUrl,
                    onValueChange = { viewModel.updateServerUrl(it) },
                    label = { Text("Server URL") },
                    placeholder = { Text("http://10.0.2.2:3000") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )

                Spacer(modifier = Modifier.height(12.dp))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Button(
                        onClick = {
                            viewModel.saveServerUrl()
                        },
                        modifier = Modifier.weight(1f),
                    ) {
                        Text("Save")
                    }

                    Button(
                        onClick = { viewModel.testConnection() },
                        modifier = Modifier.weight(1f),
                        enabled = connectionStatus !is ConnectionStatus.Testing,
                    ) {
                        Text("Test Connection")
                    }
                }

                Spacer(modifier = Modifier.height(12.dp))

                ConnectionStatusIndicator(status = connectionStatus)
            }
        }
    }
}

@Composable
private fun ConnectionStatusIndicator(status: ConnectionStatus) {
    when (status) {
        is ConnectionStatus.Idle -> {
            // Show nothing
        }
        is ConnectionStatus.Testing -> {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                CircularProgressIndicator(modifier = Modifier.size(16.dp))
                Text(
                    text = "Testing connection...",
                    style = MaterialTheme.typography.bodyMedium,
                    color = BucketOrange,
                )
            }
        }
        is ConnectionStatus.Connected -> {
            Text(
                text = "Connected - Status: ${status.status}",
                style = MaterialTheme.typography.bodyMedium,
                color = BucketGreen,
            )
        }
        is ConnectionStatus.Failed -> {
            Text(
                text = "Connection failed: ${status.message}",
                style = MaterialTheme.typography.bodyMedium,
                color = BucketRed,
            )
        }
    }
}
