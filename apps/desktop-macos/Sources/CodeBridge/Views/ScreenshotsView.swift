// Screenshots View

import SwiftUI

struct ScreenshotsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var searchText = ""

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                TextField("Search by OCR text...", text: $searchText)
                    .textFieldStyle(.roundedBorder)
                    .frame(maxWidth: 300)

                Spacer()

                Button(action: { watchScreenshotFolder() }) {
                    Label("Watch Folder", systemImage: "folder.badge.plus")
                }
                .buttonStyle(.bordered)
            }
            .padding()

            Divider()

            if bridgeManager.screenshots.isEmpty {
                VStack(spacing: 16) {
                    Image(systemName: "photo.stack")
                        .font(.system(size: 48))
                        .foregroundColor(.secondary)
                    Text("No screenshots captured")
                        .font(.title2)
                    Text("Screenshots will appear here when captured")
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ScrollView {
                    LazyVGrid(columns: [
                        GridItem(.adaptive(minimum: 200, maximum: 250))
                    ], spacing: 16) {
                        ForEach(bridgeManager.screenshots) { screenshot in
                            ScreenshotCard(screenshot: screenshot)
                        }
                    }
                    .padding()
                }
            }
        }
        .navigationTitle("Screenshots")
    }

    func watchScreenshotFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.message = "Select a folder to watch for screenshots"

        panel.begin { response in
            if response == .OK, let url = panel.url {
                // TODO: Add to watch paths
                print("Watching: \(url.path)")
            }
        }
    }
}

struct ScreenshotCard: View {
    let screenshot: Screenshot

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Thumbnail placeholder
            Rectangle()
                .fill(
                    LinearGradient(
                        colors: [.purple, .blue],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
                .aspectRatio(16/9, contentMode: .fit)
                .cornerRadius(8)

            Text(screenshot.name)
                .font(.caption)
                .fontWeight(.medium)
                .lineLimit(1)

            Text(screenshot.capturedAt, style: .relative)
                .font(.caption2)
                .foregroundColor(.secondary)
        }
        .padding(8)
        .background(.regularMaterial)
        .cornerRadius(12)
    }
}
