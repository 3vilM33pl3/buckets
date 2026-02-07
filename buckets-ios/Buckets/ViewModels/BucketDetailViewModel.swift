// SPDX-License-Identifier: MIT

import Foundation

@Observable
final class BucketDetailViewModel {
    let bucketId: UUID

    var commits: [CommitRow] = []
    var expectations: [ExpectationRow] = []
    var pebbles: [PebbleRow] = []
    var isLoading = false
    var errorMessage: String?

    init(bucketId: UUID) {
        self.bucketId = bucketId
    }

    func load() async {
        isLoading = true
        errorMessage = nil
        do {
            async let fetchedCommits = APIClient.shared.fetchCommits(bucketId: bucketId)
            async let fetchedExpectations = APIClient.shared.fetchExpectationsForBucket(id: bucketId)
            async let fetchedPebbles = APIClient.shared.fetchPebblesForBucket(id: bucketId)

            commits = try await fetchedCommits
            expectations = try await fetchedExpectations
            pebbles = try await fetchedPebbles
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
    }
}
