// SPDX-License-Identifier: MIT

import SwiftUI

struct PebblesListView: View {
    @State private var viewModel = PebblesViewModel()

    var body: some View {
        NavigationStack {
            Group {
                if viewModel.isLoading && viewModel.pebbles.isEmpty {
                    LoadingView()
                } else if let error = viewModel.errorMessage, viewModel.pebbles.isEmpty {
                    ErrorView(message: error) {
                        Task { await viewModel.load() }
                    }
                } else if viewModel.pebbles.isEmpty {
                    ContentUnavailableView(
                        "No Pebbles",
                        systemImage: "circle.grid.3x3",
                        description: Text("Pebbles will appear here once created.")
                    )
                } else {
                    List(viewModel.pebbles) { pebble in
                        HStack {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(pebble.description)
                                    .font(.subheadline)
                                    .lineLimit(2)

                                HStack(spacing: 4) {
                                    Text(pebble.originBucket)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                    Image(systemName: "arrow.right")
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                    Text(pebble.currentBucket)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }

                                Text(pebble.createdAt.formatted(date: .abbreviated, time: .shortened))
                                    .font(.caption2)
                                    .foregroundStyle(.tertiary)
                            }

                            Spacer()
                            StatusBadge(status: pebble.status)
                        }
                        .padding(.vertical, 2)
                    }
                }
            }
            .navigationTitle("Pebbles")
            .refreshable {
                await viewModel.load()
            }
            .task {
                await viewModel.load()
            }
        }
    }
}

#Preview {
    PebblesListView()
}
