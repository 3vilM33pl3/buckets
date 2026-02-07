// SPDX-License-Identifier: MIT

import Foundation

@Observable
final class SearchViewModel {
    var query = ""
    var results: [SemanticResult] = []
    var isLoading = false
    var errorMessage: String?

    func search() async {
        guard !query.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        isLoading = true
        errorMessage = nil
        do {
            results = try await APIClient.shared.search(query: query)
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
    }
}
