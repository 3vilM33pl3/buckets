// SPDX-License-Identifier: MIT

import SwiftUI

struct CommitHistoryView: View {
    let commits: [CommitRow]

    var body: some View {
        List(commits) { commit in
            NavigationLink(value: commit) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(commit.message)
                        .font(.subheadline)
                        .lineLimit(2)

                    HStack(spacing: 12) {
                        Label("\(commit.fileCount) files", systemImage: "doc")
                        Text(commit.createdAt.formatted(date: .abbreviated, time: .shortened))
                    }
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                .padding(.vertical, 2)
            }
        }
        .navigationTitle("Commits")
        .navigationDestination(for: CommitRow.self) { commit in
            CommitDetailView(commit: commit)
        }
    }
}

#Preview {
    NavigationStack {
        CommitHistoryView(commits: [
            CommitRow(
                id: UUID(),
                bucketName: "art",
                message: "Initial commit with textures",
                fileCount: 12,
                createdAt: Date()
            )
        ])
    }
}
