// SPDX-License-Identifier: MIT

import Foundation

@Observable
final class ExpectationsViewModel {
    var expectations: [ExpectationRow] = []
    var isLoading = false
    var errorMessage: String?

    func load() async {
        isLoading = true
        errorMessage = nil
        do {
            expectations = try await APIClient.shared.fetchExpectations()
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
    }
}
