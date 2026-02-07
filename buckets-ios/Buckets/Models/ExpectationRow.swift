// SPDX-License-Identifier: MIT

import Foundation

struct ExpectationRow: Codable, Identifiable {
    let id: UUID
    let description: String
    let status: String
    let sourceBucket: String
    let targetBucket: String?
    let createdAt: Date

    enum CodingKeys: String, CodingKey {
        case id
        case description
        case status
        case sourceBucket = "source_bucket"
        case targetBucket = "target_bucket"
        case createdAt = "created_at"
    }
}
