// SPDX-License-Identifier: MIT

import SwiftUI

struct BucketDetailView: View {
    let bucket: BucketRow
    @State private var viewModel: BucketDetailViewModel

    init(bucket: BucketRow) {
        self.bucket = bucket
        self._viewModel = State(initialValue: BucketDetailViewModel(bucketId: bucket.id))
    }

    var body: some View {
        Group {
            if viewModel.isLoading && viewModel.commits.isEmpty {
                LoadingView()
            } else if let error = viewModel.errorMessage,
                      viewModel.commits.isEmpty && viewModel.expectations.isEmpty {
                ErrorView(message: error) {
                    Task { await viewModel.load() }
                }
            } else {
                List {
                    // Info section
                    Section("Info") {
                        LabeledContent("Name", value: bucket.name)
                        LabeledContent("Path", value: bucket.path)
                        LabeledContent("Created", value: bucket.createdAt.formatted(date: .abbreviated, time: .shortened))
                    }

                    // Commits section
                    Section("Commits (\(viewModel.commits.count))") {
                        if viewModel.commits.isEmpty {
                            Text("No commits")
                                .foregroundStyle(.secondary)
                        } else {
                            NavigationLink("View All Commits") {
                                CommitHistoryView(commits: viewModel.commits)
                            }
                            ForEach(viewModel.commits.prefix(3)) { commit in
                                NavigationLink(value: commit) {
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text(commit.message)
                                            .font(.subheadline)
                                            .lineLimit(1)
                                        Text("\(commit.fileCount) files")
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                    }
                                }
                            }
                        }
                    }

                    // Expectations section
                    Section("Expectations (\(viewModel.expectations.count))") {
                        if viewModel.expectations.isEmpty {
                            Text("No expectations")
                                .foregroundStyle(.secondary)
                        } else {
                            ForEach(viewModel.expectations) { expectation in
                                HStack {
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text(expectation.description)
                                            .font(.subheadline)
                                            .lineLimit(2)
                                        if let target = expectation.targetBucket {
                                            Text("\(expectation.sourceBucket) -> \(target)")
                                                .font(.caption)
                                                .foregroundStyle(.secondary)
                                        }
                                    }
                                    Spacer()
                                    StatusBadge(status: expectation.status)
                                }
                            }
                        }
                    }

                    // Pebbles section
                    Section("Pebbles (\(viewModel.pebbles.count))") {
                        if viewModel.pebbles.isEmpty {
                            Text("No pebbles")
                                .foregroundStyle(.secondary)
                        } else {
                            ForEach(viewModel.pebbles) { pebble in
                                HStack {
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text(pebble.description)
                                            .font(.subheadline)
                                            .lineLimit(2)
                                        Text("\(pebble.originBucket) -> \(pebble.currentBucket)")
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                    }
                                    Spacer()
                                    StatusBadge(status: pebble.status)
                                }
                            }
                        }
                    }
                }
                .navigationDestination(for: CommitRow.self) { commit in
                    CommitDetailView(commit: commit)
                }
            }
        }
        .navigationTitle(bucket.name)
        .refreshable {
            await viewModel.load()
        }
        .task {
            await viewModel.load()
        }
    }
}

// Hashable conformance for NavigationLink
extension CommitRow: Hashable {
    static func == (lhs: CommitRow, rhs: CommitRow) -> Bool {
        lhs.id == rhs.id
    }

    func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }
}

#Preview {
    NavigationStack {
        BucketDetailView(bucket: BucketRow(
            id: UUID(),
            name: "Preview Bucket",
            path: "/example/path",
            createdAt: Date(),
            commitCount: 5,
            expectationCount: 3
        ))
    }
}
