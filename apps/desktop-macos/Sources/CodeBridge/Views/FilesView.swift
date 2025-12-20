// Files View

import SwiftUI

struct FilesView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var searchText = ""
    @State private var selectedFiles: Set<String> = []

    var filteredFiles: [TrackedFile] {
        if searchText.isEmpty {
            return bridgeManager.files
        }
        return bridgeManager.files.filter {
            $0.name.localizedCaseInsensitiveContains(searchText) ||
            $0.path.localizedCaseInsensitiveContains(searchText)
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack {
                Button(action: { bridgeManager.addFiles() }) {
                    Label("Add Files", systemImage: "plus")
                }
                .buttonStyle(.bordered)

                Spacer()

                TextField("Search files...", text: $searchText)
                    .textFieldStyle(.roundedBorder)
                    .frame(maxWidth: 300)
            }
            .padding()

            Divider()

            if filteredFiles.isEmpty {
                VStack(spacing: 16) {
                    Image(systemName: "folder")
                        .font(.system(size: 48))
                        .foregroundColor(.secondary)
                    Text("No files tracked")
                        .font(.title2)
                    Text("Add files or directories to start syncing")
                        .foregroundColor(.secondary)
                    Button("Add Files") {
                        bridgeManager.addFiles()
                    }
                    .buttonStyle(.borderedProminent)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(filteredFiles, selection: $selectedFiles) { file in
                    FileRow(file: file)
                        .contextMenu {
                            Button("Share") { bridgeManager.shareFile(file) }
                            Button("Show in Finder") { showInFinder(file.path) }
                            Divider()
                            Button("Remove", role: .destructive) { }
                        }
                }
            }
        }
        .navigationTitle("Files")
        .toolbar {
            ToolbarItem {
                Button(action: { bridgeManager.sync() }) {
                    Image(systemName: "arrow.triangle.2.circlepath")
                }
            }
        }
    }

    func showInFinder(_ path: String) {
        NSWorkspace.shared.selectFile(path, inFileViewerRootedAtPath: "")
    }
}

struct FileRow: View {
    let file: TrackedFile

    var body: some View {
        HStack {
            Image(systemName: iconForFile(file.name))
                .foregroundColor(.accentColor)
                .frame(width: 24)

            VStack(alignment: .leading) {
                Text(file.name)
                    .fontWeight(.medium)
                Text(file.path)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Spacer()

            Text(formatSize(file.size))
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.vertical, 4)
    }

    func iconForFile(_ name: String) -> String {
        let ext = (name as NSString).pathExtension.lowercased()
        switch ext {
        case "swift": return "swift"
        case "rs": return "chevron.left.forwardslash.chevron.right"
        case "js", "ts": return "doc.text"
        case "py": return "doc.text"
        case "json": return "curlybraces"
        case "md": return "doc.richtext"
        case "png", "jpg", "jpeg": return "photo"
        default: return "doc"
        }
    }

    func formatSize(_ bytes: Int64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .file
        return formatter.string(fromByteCount: bytes)
    }
}

#Preview {
    FilesView()
        .environmentObject(BridgeManager())
}
