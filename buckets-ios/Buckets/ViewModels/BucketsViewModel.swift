// SPDX-License-Identifier: MIT

import Foundation

@Observable
final class BucketsViewModel {
    var buckets: [BucketRow] = []
    var isLoading = false
    var errorMessage: String?

    func load() async {
        isLoading = true
        errorMessage = nil
        do {
            buckets = try await APIClient.shared.fetchBuckets()
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
    }
}
