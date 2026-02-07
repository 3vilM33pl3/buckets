// SPDX-License-Identifier: MIT

import Foundation

struct SemanticResult: Codable, Identifiable {
    let expectation: ExpectationRow
    let similarity: Double

    var id: UUID { expectation.id }
}
