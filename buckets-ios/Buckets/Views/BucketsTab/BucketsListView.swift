// SPDX-License-Identifier: MIT

import SwiftUI

struct BucketsListView: View {
    @State private var viewModel = BucketsViewModel()
    @State private var searchText = ""

    private var filteredBuckets: [BucketRow] {
        if searchText.isEmpty {
            return viewModel.buckets
        }
        return viewModel.buckets.filter { bucket in
            bucket.name.localizedCaseInsensitiveContains(searchText)
            || bucket.path.localizedCaseInsensitiveContains(searchText)
        }
    }

    var body: some View {
        NavigationStack {
            Group {
                if viewModel.isLoading && viewModel.buckets.isEmpty {
                    LoadingView()
                } else if let error = viewModel.errorMessage, viewModel.buckets.isEmpty {
                    ErrorView(message: error) {
                        Task { await viewModel.load() }
                    }
                } else {
                    List(filteredBuckets) { bucket in
                        NavigationLink(value: bucket) {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(bucket.name)
                                    .font(.headline)

                                Text(bucket.path)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)

                                HStack(spacing: 16) {
                                    Label("\(bucket.commitCount)", systemImage: "arrow.triangle.branch")
                                    Label("\(bucket.expectationCount)", systemImage: "checklist")
                                }
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                            }
                            .padding(.vertical, 2)
                        }
                    }
                    .navigationDestination(for: BucketRow.self) { bucket in
                        BucketDetailView(bucket: bucket)
                    }
                }
            }
            .navigationTitle("Buckets")
            .searchable(text: $searchText, prompt: "Search buckets")
            .refreshable {
                await viewModel.load()
            }
            .task {
                await viewModel.load()
            }
        }
    }
}

// Hashable conformance for NavigationLink
extension BucketRow: Hashable {
    static func == (lhs: BucketRow, rhs: BucketRow) -> Bool {
        lhs.id == rhs.id
    }

    func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }
}

#Preview {
    BucketsListView()
}
