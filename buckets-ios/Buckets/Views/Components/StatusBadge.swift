// SPDX-License-Identifier: MIT

import SwiftUI

struct StatusBadge: View {
    let status: String

    private var color: Color {
        switch status.lowercased() {
        case "pending":
            return .orange
        case "fulfilled", "complete":
            return .green
        case "rejected":
            return .red
        default:
            return .gray
        }
    }

    var body: some View {
        Text(status.capitalized)
            .font(.caption)
            .fontWeight(.medium)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(color.opacity(0.15))
            .foregroundStyle(color)
            .clipShape(Capsule())
    }
}

#Preview {
    HStack {
        StatusBadge(status: "pending")
        StatusBadge(status: "fulfilled")
        StatusBadge(status: "rejected")
        StatusBadge(status: "unknown")
    }
}
