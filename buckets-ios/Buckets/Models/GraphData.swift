// SPDX-License-Identifier: MIT

import Foundation

struct GraphData: Codable {
    let nodes: [GraphNode]
    let edges: [GraphEdge]
}

struct GraphNode: Codable, Identifiable {
    let id: UUID
    let name: String
}

struct GraphEdge: Codable, Identifiable {
    let from: UUID
    let to: UUID
    let count: Int

    var id: String { "\(from)-\(to)" }

    enum CodingKeys: String, CodingKey {
        case from
        case to
        case count
    }
}
