// SPDX-License-Identifier: MIT

import Foundation

struct BucketRow: Codable, Identifiable {
    let id: UUID
    let name: String
    let path: String
    let createdAt: Date
    let commitCount: Int
    let expectationCount: Int

    enum CodingKeys: String, CodingKey {
        case id
        case name
        case path
        case createdAt = "created_at"
        case commitCount = "commit_count"
        case expectationCount = "expectation_count"
    }
}
