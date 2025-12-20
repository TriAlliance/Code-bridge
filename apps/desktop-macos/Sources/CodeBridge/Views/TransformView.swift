// Transform View - File Conversion

import SwiftUI
import UniformTypeIdentifiers

struct TransformView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var selectedTab = 0

    var body: some View {
        VStack(spacing: 0) {
            Picker("Category", selection: $selectedTab) {
                Text("Images").tag(0)
                Text("Data").tag(1)
                Text("Documents").tag(2)
            }
            .pickerStyle(.segmented)
            .padding()

            Divider()

            switch selectedTab {
            case 0:
                ImageTransformView()
            case 1:
                DataTransformView()
            case 2:
                DocumentTransformView()
            default:
                EmptyView()
            }
        }
        .navigationTitle("Transform")
    }
}

struct ImageTransformView: View {
    @State private var sourceImage: NSImage?
    @State private var sourceURL: URL?
    @State private var targetFormat: ImageFormat = .webp
    @State private var quality: Double = 85
    @State private var resizeEnabled = false
    @State private var targetWidth: String = "800"
    @State private var targetHeight: String = "600"
    @State private var preserveAspect = true
    @State private var isConverting = false
    @State private var resultMessage: String?

    var body: some View {
        HSplitView {
            // Source
            VStack {
                Text("Source Image")
                    .font(.headline)

                ZStack {
                    if let image = sourceImage {
                        Image(nsImage: image)
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                    } else {
                        DropZoneView(
                            text: "Drop image here or click to select",
                            icon: "photo"
                        ) { urls in
                            if let url = urls.first {
                                loadImage(from: url)
                            }
                        }
                    }
                }
                .frame(maxHeight: .infinity)
                .background(Color(NSColor.controlBackgroundColor))
                .cornerRadius(8)
                .padding()

                if sourceURL != nil {
                    Button("Clear") {
                        sourceImage = nil
                        sourceURL = nil
                    }
                    .buttonStyle(.bordered)
                }
            }
            .frame(minWidth: 300)

            // Options & Output
            VStack {
                Form {
                    Section("Output Format") {
                        Picker("Format", selection: $targetFormat) {
                            ForEach(ImageFormat.allCases, id: \.self) { format in
                                Text(format.displayName).tag(format)
                            }
                        }

                        if targetFormat.supportsQuality {
                            Slider(value: $quality, in: 1...100, step: 1) {
                                Text("Quality")
                            }
                            Text("\(Int(quality))%")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }

                    Section("Resize") {
                        Toggle("Resize Image", isOn: $resizeEnabled)

                        if resizeEnabled {
                            HStack {
                                TextField("Width", text: $targetWidth)
                                    .textFieldStyle(.roundedBorder)
                                    .frame(width: 80)
                                Text("x")
                                TextField("Height", text: $targetHeight)
                                    .textFieldStyle(.roundedBorder)
                                    .frame(width: 80)
                                Text("px")
                            }

                            Toggle("Preserve Aspect Ratio", isOn: $preserveAspect)
                        }
                    }
                }
                .formStyle(.grouped)
                .frame(maxWidth: 300)

                Spacer()

                if let message = resultMessage {
                    Text(message)
                        .foregroundColor(.green)
                        .padding()
                }

                Button(action: { convertImage() }) {
                    if isConverting {
                        ProgressView()
                            .scaleEffect(0.5)
                    } else {
                        Label("Convert", systemImage: "arrow.triangle.2.circlepath")
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(sourceURL == nil || isConverting)
                .padding()
            }
        }
    }

    func loadImage(from url: URL) {
        if let image = NSImage(contentsOf: url) {
            sourceImage = image
            sourceURL = url
        }
    }

    func convertImage() {
        guard let url = sourceURL else { return }
        isConverting = true
        resultMessage = nil

        // Simulate conversion
        DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
            isConverting = false
            resultMessage = "Converted successfully!"
        }
    }
}

struct DataTransformView: View {
    @State private var sourceText = ""
    @State private var outputText = ""
    @State private var sourceFormat: DataFormat = .json
    @State private var targetFormat: DataFormat = .yaml
    @State private var prettyPrint = true
    @State private var errorMessage: String?

    var body: some View {
        VStack(spacing: 0) {
            // Format selectors
            HStack {
                Picker("From", selection: $sourceFormat) {
                    ForEach(DataFormat.allCases, id: \.self) { format in
                        Text(format.rawValue.uppercased()).tag(format)
                    }
                }
                .frame(width: 120)

                Image(systemName: "arrow.right")

                Picker("To", selection: $targetFormat) {
                    ForEach(DataFormat.allCases, id: \.self) { format in
                        Text(format.rawValue.uppercased()).tag(format)
                    }
                }
                .frame(width: 120)

                Toggle("Pretty Print", isOn: $prettyPrint)

                Spacer()

                Button("Convert") {
                    convert()
                }
                .buttonStyle(.borderedProminent)

                Button("Swap") {
                    swap(&sourceFormat, &targetFormat)
                    swap(&sourceText, &outputText)
                }
                .buttonStyle(.bordered)
            }
            .padding()

            if let error = errorMessage {
                Text(error)
                    .foregroundColor(.red)
                    .padding(.horizontal)
            }

            Divider()

            HSplitView {
                // Source
                VStack(alignment: .leading) {
                    Text("Input (\(sourceFormat.rawValue.uppercased()))")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.horizontal)
                        .padding(.top, 8)

                    TextEditor(text: $sourceText)
                        .font(.system(.body, design: .monospaced))
                        .padding(4)
                }

                // Output
                VStack(alignment: .leading) {
                    HStack {
                        Text("Output (\(targetFormat.rawValue.uppercased()))")
                            .font(.caption)
                            .foregroundColor(.secondary)

                        Spacer()

                        Button(action: { copyOutput() }) {
                            Image(systemName: "doc.on.doc")
                        }
                        .buttonStyle(.plain)
                    }
                    .padding(.horizontal)
                    .padding(.top, 8)

                    TextEditor(text: .constant(outputText))
                        .font(.system(.body, design: .monospaced))
                        .padding(4)
                }
            }
        }
    }

    func convert() {
        errorMessage = nil

        // Simple conversion demo
        do {
            if sourceFormat == .json && targetFormat == .yaml {
                // JSON to YAML
                if let data = sourceText.data(using: .utf8),
                   let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] {
                    outputText = convertToYAML(json)
                }
            } else if sourceFormat == .yaml && targetFormat == .json {
                // YAML to JSON - simplified
                outputText = "{\n  // YAML to JSON conversion\n}"
            } else {
                outputText = sourceText
            }
        } catch {
            errorMessage = "Conversion failed: \(error.localizedDescription)"
        }
    }

    func convertToYAML(_ dict: [String: Any], indent: Int = 0) -> String {
        var result = ""
        let prefix = String(repeating: "  ", count: indent)

        for (key, value) in dict {
            if let nested = value as? [String: Any] {
                result += "\(prefix)\(key):\n"
                result += convertToYAML(nested, indent: indent + 1)
            } else if let array = value as? [Any] {
                result += "\(prefix)\(key):\n"
                for item in array {
                    result += "\(prefix)  - \(item)\n"
                }
            } else {
                result += "\(prefix)\(key): \(value)\n"
            }
        }

        return result
    }

    func copyOutput() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(outputText, forType: .string)
    }
}

struct DocumentTransformView: View {
    @State private var sourceText = ""
    @State private var outputText = ""
    @State private var sourceFormat: DocFormat = .markdown
    @State private var targetFormat: DocFormat = .html
    @State private var showPreview = true

    var body: some View {
        VStack(spacing: 0) {
            // Format selectors
            HStack {
                Picker("From", selection: $sourceFormat) {
                    ForEach(DocFormat.allCases, id: \.self) { format in
                        Text(format.displayName).tag(format)
                    }
                }
                .frame(width: 150)

                Image(systemName: "arrow.right")

                Picker("To", selection: $targetFormat) {
                    ForEach(DocFormat.allCases, id: \.self) { format in
                        Text(format.displayName).tag(format)
                    }
                }
                .frame(width: 150)

                Spacer()

                Toggle("Preview", isOn: $showPreview)

                Button("Convert") {
                    convert()
                }
                .buttonStyle(.borderedProminent)
            }
            .padding()

            Divider()

            HSplitView {
                // Source
                VStack(alignment: .leading) {
                    Text("Input")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.horizontal)
                        .padding(.top, 8)

                    TextEditor(text: $sourceText)
                        .font(.system(.body, design: .monospaced))
                        .padding(4)
                }

                // Output
                VStack(alignment: .leading) {
                    HStack {
                        Text("Output")
                            .font(.caption)
                            .foregroundColor(.secondary)

                        Spacer()

                        Button(action: { copyOutput() }) {
                            Image(systemName: "doc.on.doc")
                        }
                        .buttonStyle(.plain)
                    }
                    .padding(.horizontal)
                    .padding(.top, 8)

                    if showPreview && targetFormat == .html {
                        // HTML Preview would go here
                        ScrollView {
                            Text(outputText)
                                .font(.system(.body, design: .monospaced))
                                .padding()
                        }
                    } else {
                        TextEditor(text: .constant(outputText))
                            .font(.system(.body, design: .monospaced))
                            .padding(4)
                    }
                }
            }
        }
    }

    func convert() {
        if sourceFormat == .markdown && targetFormat == .html {
            // Simple markdown to HTML (basic implementation)
            var html = sourceText

            // Headers
            html = html.replacingOccurrences(of: "^# (.+)$", with: "<h1>$1</h1>", options: .regularExpression)
            html = html.replacingOccurrences(of: "^## (.+)$", with: "<h2>$1</h2>", options: .regularExpression)
            html = html.replacingOccurrences(of: "^### (.+)$", with: "<h3>$1</h3>", options: .regularExpression)

            // Bold and italic
            html = html.replacingOccurrences(of: "\\*\\*(.+?)\\*\\*", with: "<strong>$1</strong>", options: .regularExpression)
            html = html.replacingOccurrences(of: "\\*(.+?)\\*", with: "<em>$1</em>", options: .regularExpression)

            // Paragraphs
            html = html.components(separatedBy: "\n\n").map { "<p>\($0)</p>" }.joined(separator: "\n")

            outputText = html
        } else {
            outputText = sourceText
        }
    }

    func copyOutput() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(outputText, forType: .string)
    }
}

struct DropZoneView: View {
    let text: String
    let icon: String
    let onDrop: ([URL]) -> Void

    @State private var isHovering = false

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: icon)
                .font(.system(size: 48))
                .foregroundColor(.secondary)

            Text(text)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(isHovering ? Color.accentColor.opacity(0.1) : Color.clear)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .strokeBorder(style: StrokeStyle(lineWidth: 2, dash: [5]))
                .foregroundColor(isHovering ? .accentColor : .secondary.opacity(0.5))
        )
        .onDrop(of: [.fileURL], isTargeted: $isHovering) { providers in
            for provider in providers {
                provider.loadObject(ofClass: URL.self) { url, _ in
                    if let url = url {
                        DispatchQueue.main.async {
                            onDrop([url])
                        }
                    }
                }
            }
            return true
        }
        .onTapGesture {
            selectFile()
        }
    }

    func selectFile() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false

        if panel.runModal() == .OK, let url = panel.url {
            onDrop([url])
        }
    }
}

// Enums
enum ImageFormat: String, CaseIterable {
    case png, jpeg, webp, gif

    var displayName: String {
        rawValue.uppercased()
    }

    var supportsQuality: Bool {
        self == .jpeg || self == .webp
    }
}

enum DataFormat: String, CaseIterable {
    case json, yaml, toml, xml
}

enum DocFormat: String, CaseIterable {
    case markdown, html, plainText

    var displayName: String {
        switch self {
        case .markdown: return "Markdown"
        case .html: return "HTML"
        case .plainText: return "Plain Text"
        }
    }
}

#Preview {
    TransformView()
        .environmentObject(BridgeManager())
}
