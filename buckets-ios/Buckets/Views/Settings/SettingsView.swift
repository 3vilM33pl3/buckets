// SPDX-License-Identifier: MIT

import SwiftUI

struct SettingsView: View {
    @AppStorage("serverURL") private var serverURL = "http://localhost:3000"
    @State private var connectionStatus: ConnectionStatus = .unknown
    @State private var isTesting = false

    enum ConnectionStatus {
        case unknown
        case connected
        case failed(String)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("Server") {
                    TextField("Server URL", text: $serverURL)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()

                    Button {
                        Task { await testConnection() }
                    } label: {
                        HStack {
                            Text("Test Connection")
                            Spacer()
                            if isTesting {
                                ProgressView()
                            } else {
                                statusIcon
                            }
                        }
                    }
                    .disabled(isTesting)
                }

                Section("About") {
                    LabeledContent("App", value: "Buckets iOS")
                    LabeledContent("Version", value: "1.0.0")
                }
            }
            .navigationTitle("Settings")
        }
    }

    @ViewBuilder
    private var statusIcon: some View {
        switch connectionStatus {
        case .unknown:
            EmptyView()
        case .connected:
            Image(systemName: "checkmark.circle.fill")
                .foregroundStyle(.green)
        case .failed:
            Image(systemName: "xmark.circle.fill")
                .foregroundStyle(.red)
        }
    }

    private func testConnection() async {
        isTesting = true
        connectionStatus = .unknown

        guard let url = URL(string: "\(serverURL)/api/health") else {
            connectionStatus = .failed("Invalid URL")
            isTesting = false
            return
        }

        do {
            let (_, response) = try await URLSession.shared.data(from: url)
            if let httpResponse = response as? HTTPURLResponse,
               (200...299).contains(httpResponse.statusCode) {
                connectionStatus = .connected
            } else {
                connectionStatus = .failed("Unexpected response")
            }
        } catch {
            connectionStatus = .failed(error.localizedDescription)
        }

        isTesting = false
    }
}

#Preview {
    SettingsView()
}
