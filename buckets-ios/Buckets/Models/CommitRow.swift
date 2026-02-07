// SPDX-License-Identifier: MIT

import Foundation

struct CommitRow: Codable, Identifiable {
    let id: UUID
    let bucketName: String
    let message: String
    let fileCount: Int
    let createdAt: Date

    enum CodingKeys: String, CodingKey {
        case id
        case bucketName = "bucket_name"
        case message
        case fileCount = "file_count"
        case createdAt = "created_at"
    }
}
