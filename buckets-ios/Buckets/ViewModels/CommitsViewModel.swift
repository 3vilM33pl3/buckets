// SPDX-License-Identifier: MIT

import Foundation

@Observable
final class CommitsViewModel {
    let commitId: UUID

    var files: [FileRow] = []
    var isLoading = false
    var errorMessage: String?

    init(commitId: UUID) {
        self.commitId = commitId
    }

    func load() async {
        isLoading = true
        errorMessage = nil
        do {
            files = try await APIClient.shared.fetchFiles(commitId: commitId)
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
    }
}
