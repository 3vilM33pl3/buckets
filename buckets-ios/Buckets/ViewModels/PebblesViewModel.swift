// SPDX-License-Identifier: MIT

import Foundation

@Observable
final class PebblesViewModel {
    var pebbles: [PebbleRow] = []
    var isLoading = false
    var errorMessage: String?

    func load() async {
        isLoading = true
        errorMessage = nil
        do {
            pebbles = try await APIClient.shared.fetchPebbles()
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
    }
}
