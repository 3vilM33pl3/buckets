// SPDX-License-Identifier: MIT

import SwiftUI

struct ExpectationsListView: View {
    @State private var viewModel = ExpectationsViewModel()
    @State private var searchViewModel = SearchViewModel()
    @State private var searchText = ""

    private var displayedExpectations: [ExpectationRow] {
        if !searchViewModel.results.isEmpty {
            return searchViewModel.results.map(\.expectation)
        }
        if searchText.isEmpty {
            return viewModel.expectations
        }
        return viewModel.expectations.filter { expectation in
            expectation.description.localizedCaseInsensitiveContains(searchText)
        }
    }

    var body: some View {
        NavigationStack {
            Group {
                if (viewModel.isLoading && viewModel.expectations.isEmpty)
                    || searchViewModel.isLoading {
                    LoadingView()
                } else if let error = viewModel.errorMessage, viewModel.expectations.isEmpty {
                    ErrorView(message: error) {
                        Task { await viewModel.load() }
                    }
                } else {
                    List {
                        if !searchViewModel.results.isEmpty {
                            Section {
                                Text("Showing semantic search results")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }

                        ForEach(displayedExpectations) { expectation in
                            HStack {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(expectation.description)
                                        .font(.subheadline)
                                        .lineLimit(2)

                                    HStack(spacing: 4) {
                                        Text(expectation.sourceBucket)
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                        if let target = expectation.targetBucket {
                                            Image(systemName: "arrow.right")
                                                .font(.caption2)
                                                .foregroundStyle(.secondary)
                                            Text(target)
                                                .font(.caption)
                                                .foregroundStyle(.secondary)
                                        }
                                    }

                                    if let result = searchViewModel.results.first(where: { $0.expectation.id == expectation.id }) {
                                        Text("Similarity: \(result.similarity, specifier: "%.1f")%")
                                            .font(.caption2)
                                            .foregroundStyle(.blue)
                                    }
                                }

                                Spacer()
                                StatusBadge(status: expectation.status)
                            }
                            .padding(.vertical, 2)
                        }
                    }
                }
            }
            .navigationTitle("Expectations")
            .searchable(text: $searchText, prompt: "Search expectations")
            .onSubmit(of: .search) {
                searchViewModel.query = searchText
                Task { await searchViewModel.search() }
            }
            .onChange(of: searchText) { _, newValue in
                if newValue.isEmpty {
                    searchViewModel.results = []
                }
            }
            .refreshable {
                searchViewModel.results = []
                await viewModel.load()
            }
            .task {
                await viewModel.load()
            }
        }
    }
}

#Preview {
    ExpectationsListView()
}
