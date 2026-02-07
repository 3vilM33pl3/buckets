// SPDX-License-Identifier: MIT

import SwiftUI

struct GraphView: View {
    @State private var viewModel = GraphViewModel()

    var body: some View {
        NavigationStack {
            Group {
                if viewModel.isLoading && viewModel.graphData == nil {
                    LoadingView()
                } else if let error = viewModel.errorMessage, viewModel.graphData == nil {
                    ErrorView(message: error) {
                        Task { await viewModel.load() }
                    }
                } else if let graph = viewModel.graphData {
                    List {
                        Section("Nodes (\(graph.nodes.count))") {
                            if graph.nodes.isEmpty {
                                Text("No nodes")
                                    .foregroundStyle(.secondary)
                            } else {
                                ForEach(graph.nodes) { node in
                                    HStack {
                                        Image(systemName: "circle.fill")
                                            .font(.caption2)
                                            .foregroundStyle(.blue)
                                        Text(node.name)
                                            .font(.subheadline)
                                    }
                                }
                            }
                        }

                        Section("Edges (\(graph.edges.count))") {
                            if graph.edges.isEmpty {
                                Text("No edges")
                                    .foregroundStyle(.secondary)
                            } else {
                                ForEach(graph.edges) { edge in
                                    HStack {
                                        let sourceName = nodeName(for: edge.from, in: graph.nodes)
                                        let targetName = nodeName(for: edge.to, in: graph.nodes)

                                        Text(sourceName)
                                            .font(.subheadline)
                                        Image(systemName: "arrow.right")
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                        Text(targetName)
                                            .font(.subheadline)

                                        Spacer()

                                        Text("\(edge.count)")
                                            .font(.caption)
                                            .padding(.horizontal, 8)
                                            .padding(.vertical, 2)
                                            .background(.blue.opacity(0.1))
                                            .foregroundStyle(.blue)
                                            .clipShape(Capsule())
                                    }
                                }
                            }
                        }
                    }
                } else {
                    ContentUnavailableView(
                        "No Graph Data",
                        systemImage: "point.3.connected.trianglepath.dotted",
                        description: Text("Graph data will appear here once buckets are connected.")
                    )
                }
            }
            .navigationTitle("Graph")
            .refreshable {
                await viewModel.load()
            }
            .task {
                await viewModel.load()
            }
        }
    }

    private func nodeName(for id: UUID, in nodes: [GraphNode]) -> String {
        nodes.first(where: { $0.id == id })?.name ?? id.uuidString.prefix(8).description
    }
}

#Preview {
    GraphView()
}
