// SPDX-License-Identifier: MIT

import Foundation

@Observable
final class GraphViewModel {
    var graphData: GraphData?
    var isLoading = false
    var errorMessage: String?

    func load() async {
        isLoading = true
        errorMessage = nil
        do {
            graphData = try await APIClient.shared.fetchGraph()
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
    }
}
