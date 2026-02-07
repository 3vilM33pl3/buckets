// SPDX-License-Identifier: MIT

import SwiftUI

@main
struct BucketsApp: App {
    var body: some Scene {
        WindowGroup {
            TabView {
                BucketsListView()
                    .tabItem {
                        Label("Buckets", systemImage: "archivebox")
                    }

                ExpectationsListView()
                    .tabItem {
                        Label("Expectations", systemImage: "checklist")
                    }

                PebblesListView()
                    .tabItem {
                        Label("Pebbles", systemImage: "circle.grid.3x3")
                    }

                GraphView()
                    .tabItem {
                        Label("Graph", systemImage: "point.3.connected.trianglepath.dotted")
                    }

                SettingsView()
                    .tabItem {
                        Label("Settings", systemImage: "gear")
                    }
            }
        }
    }
}
