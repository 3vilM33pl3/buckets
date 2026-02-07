// SPDX-License-Identifier: MIT

import Foundation

struct PebbleRow: Codable, Identifiable {
    let id: UUID
    let description: String
    let originBucket: String
    let currentBucket: String
    let status: String
    let createdAt: Date

    enum CodingKeys: String, CodingKey {
        case id
        case description
        case originBucket = "origin_bucket"
        case currentBucket = "current_bucket"
        case status
        case createdAt = "created_at"
    }
}
