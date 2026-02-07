// SPDX-License-Identifier: MIT

import SwiftUI

struct CommitDetailView: View {
    let commit: CommitRow
    @State private var viewModel: CommitsViewModel

    init(commit: CommitRow) {
        self.commit = commit
        self._viewModel = State(initialValue: CommitsViewModel(commitId: commit.id))
    }

    var body: some View {
        Group {
            if viewModel.isLoading && viewModel.files.isEmpty {
                LoadingView()
            } else if let error = viewModel.errorMessage, viewModel.files.isEmpty {
                ErrorView(message: error) {
                    Task { await viewModel.load() }
                }
            } else {
                List {
                    Section("Commit Info") {
                        LabeledContent("Message", value: commit.message)
                        LabeledContent("Bucket", value: commit.bucketName)
                        LabeledContent("Date", value: commit.createdAt.formatted(date: .abbreviated, time: .shortened))
                        LabeledContent("Files", value: "\(commit.fileCount)")
                    }

                    Section("Files (\(viewModel.files.count))") {
                        if viewModel.files.isEmpty {
                            Text("No files")
                                .foregroundStyle(.secondary)
                        } else {
                            ForEach(viewModel.files) { file in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(file.filePath)
                                        .font(.subheadline)
                                        .lineLimit(1)
                                    Text(file.hash)
                                        .font(.caption2)
                                        .monospaced()
                                        .foregroundStyle(.secondary)
                                        .lineLimit(1)
                                }
                                .padding(.vertical, 2)
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle("Commit")
        .task {
            await viewModel.load()
        }
    }
}

#Preview {
    NavigationStack {
        CommitDetailView(commit: CommitRow(
            id: UUID(),
            bucketName: "art",
            message: "Add character textures",
            fileCount: 5,
            createdAt: Date()
        ))
    }
}
