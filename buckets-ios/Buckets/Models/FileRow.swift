// SPDX-License-Identifier: MIT

import Foundation

struct FileRow: Codable, Identifiable {
    let id: UUID
    let filePath: String
    let hash: String

    enum CodingKeys: String, CodingKey {
        case id
        case filePath = "file_path"
        case hash
    }
}
