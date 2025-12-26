// QNAPService - QNAP NAS API Integration
// Implements File Station API v5 for file operations and backup

import Foundation

/// Service for interacting with QNAP NAS via File Station API
@MainActor
class QNAPService: ObservableObject {
    // MARK: - Published State
    @Published var connectionStatus: NASConnectionStatus = .disconnected
    @Published var sessionId: String?
    @Published var lastError: String?
    @Published var availableShares: [String] = []
    @Published var currentPath: String = "/"
    @Published var directoryContents: [QNAPFileInfo] = []

    // MARK: - Private Properties
    private var activeConnection: QNAPConnection?
    private let session: URLSession
    private var sessionExpiryTimer: Timer?

    init() {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 30
        config.timeoutIntervalForResource = 300
        self.session = URLSession(configuration: config)
    }

    // MARK: - Connection Management

    /// Connect to QNAP NAS using the provided connection settings
    func connect(using connection: QNAPConnection) async throws {
        activeConnection = connection
        connectionStatus = .connecting
        lastError = nil

        do {
            switch connection.qnapProtocol {
            case .api:
                try await authenticateViaAPI(connection)
            case .webdav:
                try await testWebDAVConnection(connection)
            case .smb3, .nfs:
                try await testMountConnection(connection)
            }
            connectionStatus = .connected
            startSessionKeepAlive()
        } catch {
            connectionStatus = .error
            lastError = error.localizedDescription
            throw error
        }
    }

    /// Disconnect from the current NAS
    func disconnect() async {
        if let sid = sessionId, let connection = activeConnection {
            // Logout from API if using API protocol
            if connection.qnapProtocol == .api {
                await logoutFromAPI(connection, sessionId: sid)
            }
        }

        sessionExpiryTimer?.invalidate()
        sessionExpiryTimer = nil
        sessionId = nil
        activeConnection = nil
        connectionStatus = .disconnected
        availableShares = []
        directoryContents = []
    }

    /// Test connection without fully authenticating
    func testConnection(_ connection: QNAPConnection) async -> (success: Bool, message: String) {
        do {
            let baseURL = buildBaseURL(for: connection)
            guard let url = URL(string: "\(baseURL)/cgi-bin/authLogin.cgi") else {
                return (false, "Invalid URL")
            }

            var request = URLRequest(url: url)
            request.httpMethod = "GET"
            request.timeoutInterval = 10

            let (_, response) = try await session.data(for: request)

            if let httpResponse = response as? HTTPURLResponse {
                if httpResponse.statusCode == 200 {
                    return (true, "Connection successful")
                } else {
                    return (false, "Server returned status \(httpResponse.statusCode)")
                }
            }
            return (false, "Unknown response")
        } catch {
            return (false, error.localizedDescription)
        }
    }

    // MARK: - File Station API Authentication

    private func authenticateViaAPI(_ connection: QNAPConnection) async throws {
        let baseURL = buildBaseURL(for: connection)

        // QNAP File Station API authentication endpoint
        guard var components = URLComponents(string: "\(baseURL)/cgi-bin/authLogin.cgi") else {
            throw QNAPError.invalidURL
        }

        // Password needs to be base64 encoded for QNAP API
        let encodedPassword = Data(connection.password.utf8).base64EncodedString()

        components.queryItems = [
            URLQueryItem(name: "user", value: connection.username),
            URLQueryItem(name: "pwd", value: encodedPassword)
        ]

        guard let url = components.url else {
            throw QNAPError.invalidURL
        }

        var request = URLRequest(url: url)
        request.httpMethod = "GET"

        let (data, response) = try await session.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw QNAPError.invalidResponse
        }

        guard httpResponse.statusCode == 200 else {
            throw QNAPError.authenticationFailed("Server returned \(httpResponse.statusCode)")
        }

        // Parse XML response to get session ID
        let responseString = String(data: data, encoding: .utf8) ?? ""

        if let sid = extractSessionId(from: responseString) {
            sessionId = sid
            // Load available shares after successful auth
            try await loadShares()
        } else if responseString.contains("error") {
            throw QNAPError.authenticationFailed("Invalid credentials")
        } else {
            throw QNAPError.authenticationFailed("Could not obtain session ID")
        }
    }

    private func logoutFromAPI(_ connection: QNAPConnection, sessionId: String) async {
        let baseURL = buildBaseURL(for: connection)
        guard let url = URL(string: "\(baseURL)/cgi-bin/authLogout.cgi?sid=\(sessionId)") else { return }

        _ = try? await session.data(from: url)
    }

    private func extractSessionId(from response: String) -> String? {
        // QNAP returns XML like: <authSid><![CDATA[abc123]]></authSid>
        if let range = response.range(of: "<authSid><![CDATA["),
           let endRange = response.range(of: "]]></authSid>") {
            let start = range.upperBound
            let end = endRange.lowerBound
            return String(response[start..<end])
        }
        return nil
    }

    // MARK: - WebDAV Connection

    private func testWebDAVConnection(_ connection: QNAPConnection) async throws {
        let scheme = connection.useSSL ? "https" : "http"
        let urlString = "\(scheme)://\(connection.host):\(connection.port)\(connection.sharePath)"

        guard let url = URL(string: urlString) else {
            throw QNAPError.invalidURL
        }

        var request = URLRequest(url: url)
        request.httpMethod = "PROPFIND"
        request.setValue("0", forHTTPHeaderField: "Depth")

        // Basic auth header
        let credentials = "\(connection.username):\(connection.password)"
        if let credData = credentials.data(using: .utf8) {
            let base64Creds = credData.base64EncodedString()
            request.setValue("Basic \(base64Creds)", forHTTPHeaderField: "Authorization")
        }

        let (_, response) = try await session.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw QNAPError.invalidResponse
        }

        // WebDAV returns 207 Multi-Status for successful PROPFIND
        guard httpResponse.statusCode == 207 || httpResponse.statusCode == 200 else {
            if httpResponse.statusCode == 401 {
                throw QNAPError.authenticationFailed("Invalid credentials")
            }
            throw QNAPError.connectionFailed("WebDAV returned \(httpResponse.statusCode)")
        }

        // WebDAV doesn't use session IDs, we'll use a marker
        sessionId = "webdav-\(UUID().uuidString)"
    }

    // MARK: - SMB/NFS Mount Connection

    private func testMountConnection(_ connection: QNAPConnection) async throws {
        // For SMB/NFS, we'll verify the share is accessible
        // In a real implementation, this would use NetFS framework

        let scheme = connection.qnapProtocol == .smb3 ? "smb" : "nfs"
        let mountPath = "/Volumes/\(connection.name)"

        // Check if already mounted
        if FileManager.default.fileExists(atPath: mountPath) {
            sessionId = "mount-\(UUID().uuidString)"
            return
        }

        // For now, simulate a successful connection test
        // In production, use NetFSMountURLSync or similar
        sessionId = "mount-\(UUID().uuidString)"
    }

    // MARK: - File Operations

    /// Load available shares from the NAS
    func loadShares() async throws {
        guard let connection = activeConnection, let sid = sessionId else {
            throw QNAPError.notConnected
        }

        if connection.qnapProtocol == .api {
            let baseURL = buildBaseURL(for: connection)
            guard let url = URL(string: "\(baseURL)/cgi-bin/filemanager/utilRequest.cgi?func=get_tree&sid=\(sid)&is_iso=0&node=share_root") else {
                throw QNAPError.invalidURL
            }

            let (data, _) = try await session.data(from: url)

            // Parse response to extract share names
            if let json = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] {
                availableShares = json.compactMap { $0["text"] as? String }
            }
        } else {
            // For WebDAV/SMB/NFS, use the configured share path
            availableShares = [connection.sharePath]
        }
    }

    /// List directory contents
    func listDirectory(path: String) async throws -> [QNAPFileInfo] {
        guard let connection = activeConnection, let sid = sessionId else {
            throw QNAPError.notConnected
        }

        currentPath = path

        if connection.qnapProtocol == .api {
            return try await listDirectoryViaAPI(path: path, connection: connection, sessionId: sid)
        } else if connection.qnapProtocol == .webdav {
            return try await listDirectoryViaWebDAV(path: path, connection: connection)
        } else {
            return try await listDirectoryViaMountPoint(path: path, connection: connection)
        }
    }

    private func listDirectoryViaAPI(path: String, connection: QNAPConnection, sessionId: String) async throws -> [QNAPFileInfo] {
        let baseURL = buildBaseURL(for: connection)
        let encodedPath = path.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? path

        guard let url = URL(string: "\(baseURL)/cgi-bin/filemanager/utilRequest.cgi?func=get_list&sid=\(sessionId)&path=\(encodedPath)&list_mode=all&sort=filename&dir=ASC") else {
            throw QNAPError.invalidURL
        }

        let (data, _) = try await session.data(from: url)

        // Parse the response
        if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
           let datas = json["datas"] as? [[String: Any]] {
            return datas.map { item in
                QNAPFileInfo(
                    name: item["filename"] as? String ?? "",
                    path: "\(path)/\(item["filename"] as? String ?? "")",
                    isDirectory: (item["isfolder"] as? Int ?? 0) == 1,
                    size: Int64(item["filesize"] as? String ?? "0") ?? 0,
                    modifiedDate: Date(),
                    owner: item["owner"] as? String ?? ""
                )
            }
        }

        return []
    }

    private func listDirectoryViaWebDAV(path: String, connection: QNAPConnection) async throws -> [QNAPFileInfo] {
        let scheme = connection.useSSL ? "https" : "http"
        let urlString = "\(scheme)://\(connection.host):\(connection.port)\(path)"

        guard let url = URL(string: urlString) else {
            throw QNAPError.invalidURL
        }

        var request = URLRequest(url: url)
        request.httpMethod = "PROPFIND"
        request.setValue("1", forHTTPHeaderField: "Depth")

        let credentials = "\(connection.username):\(connection.password)"
        if let credData = credentials.data(using: .utf8) {
            request.setValue("Basic \(credData.base64EncodedString())", forHTTPHeaderField: "Authorization")
        }

        let (data, _) = try await session.data(for: request)

        // Parse WebDAV XML response (simplified)
        return parseWebDAVResponse(data, basePath: path)
    }

    private func listDirectoryViaMountPoint(path: String, connection: QNAPConnection) async throws -> [QNAPFileInfo] {
        let mountPath = "/Volumes/\(connection.name)\(path)"
        let fileManager = FileManager.default

        guard fileManager.fileExists(atPath: mountPath) else {
            throw QNAPError.pathNotFound(path)
        }

        let contents = try fileManager.contentsOfDirectory(atPath: mountPath)

        return try contents.map { name in
            let fullPath = "\(mountPath)/\(name)"
            let attrs = try fileManager.attributesOfItem(atPath: fullPath)

            return QNAPFileInfo(
                name: name,
                path: "\(path)/\(name)",
                isDirectory: attrs[.type] as? FileAttributeType == .typeDirectory,
                size: attrs[.size] as? Int64 ?? 0,
                modifiedDate: attrs[.modificationDate] as? Date ?? Date(),
                owner: ""
            )
        }
    }

    private func parseWebDAVResponse(_ data: Data, basePath: String) -> [QNAPFileInfo] {
        // Simplified WebDAV XML parsing
        // In production, use proper XML parsing
        var files: [QNAPFileInfo] = []

        if let content = String(data: data, encoding: .utf8) {
            // Extract href elements (simplified)
            let pattern = "<D:href>([^<]+)</D:href>"
            if let regex = try? NSRegularExpression(pattern: pattern) {
                let range = NSRange(content.startIndex..., in: content)
                let matches = regex.matches(in: content, range: range)

                for match in matches {
                    if let hrefRange = Range(match.range(at: 1), in: content) {
                        let href = String(content[hrefRange])
                        let name = URL(string: href)?.lastPathComponent ?? href
                        if !name.isEmpty && name != basePath.split(separator: "/").last.map(String.init) {
                            files.append(QNAPFileInfo(
                                name: name,
                                path: "\(basePath)/\(name)",
                                isDirectory: href.hasSuffix("/"),
                                size: 0,
                                modifiedDate: Date(),
                                owner: ""
                            ))
                        }
                    }
                }
            }
        }

        return files
    }

    /// Upload a file to the NAS
    func uploadFile(localPath: String, remotePath: String, progress: @escaping (Double) -> Void) async throws {
        guard let connection = activeConnection, let sid = sessionId else {
            throw QNAPError.notConnected
        }

        let fileURL = URL(fileURLWithPath: localPath)
        guard FileManager.default.fileExists(atPath: localPath) else {
            throw QNAPError.fileNotFound(localPath)
        }

        if connection.qnapProtocol == .api {
            try await uploadViaAPI(fileURL: fileURL, remotePath: remotePath, connection: connection, sessionId: sid, progress: progress)
        } else if connection.qnapProtocol == .webdav {
            try await uploadViaWebDAV(fileURL: fileURL, remotePath: remotePath, connection: connection, progress: progress)
        } else {
            try await uploadViaMountPoint(fileURL: fileURL, remotePath: remotePath, connection: connection, progress: progress)
        }
    }

    private func uploadViaAPI(fileURL: URL, remotePath: String, connection: QNAPConnection, sessionId: String, progress: @escaping (Double) -> Void) async throws {
        let baseURL = buildBaseURL(for: connection)

        guard let url = URL(string: "\(baseURL)/cgi-bin/filemanager/utilRequest.cgi?func=upload&sid=\(sessionId)&dest_path=\(remotePath)") else {
            throw QNAPError.invalidURL
        }

        let boundary = UUID().uuidString
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("multipart/form-data; boundary=\(boundary)", forHTTPHeaderField: "Content-Type")

        let fileData = try Data(contentsOf: fileURL)
        var body = Data()

        body.append("--\(boundary)\r\n".data(using: .utf8)!)
        body.append("Content-Disposition: form-data; name=\"file\"; filename=\"\(fileURL.lastPathComponent)\"\r\n".data(using: .utf8)!)
        body.append("Content-Type: application/octet-stream\r\n\r\n".data(using: .utf8)!)
        body.append(fileData)
        body.append("\r\n--\(boundary)--\r\n".data(using: .utf8)!)

        request.httpBody = body

        progress(0.5) // Simulated progress

        let (_, response) = try await session.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse, httpResponse.statusCode == 200 else {
            throw QNAPError.uploadFailed("Upload failed")
        }

        progress(1.0)
    }

    private func uploadViaWebDAV(fileURL: URL, remotePath: String, connection: QNAPConnection, progress: @escaping (Double) -> Void) async throws {
        let scheme = connection.useSSL ? "https" : "http"
        let destPath = "\(remotePath)/\(fileURL.lastPathComponent)"
        let urlString = "\(scheme)://\(connection.host):\(connection.port)\(destPath)"

        guard let url = URL(string: urlString) else {
            throw QNAPError.invalidURL
        }

        var request = URLRequest(url: url)
        request.httpMethod = "PUT"

        let credentials = "\(connection.username):\(connection.password)"
        if let credData = credentials.data(using: .utf8) {
            request.setValue("Basic \(credData.base64EncodedString())", forHTTPHeaderField: "Authorization")
        }

        let fileData = try Data(contentsOf: fileURL)
        request.httpBody = fileData

        progress(0.5)

        let (_, response) = try await session.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse,
              (200...299).contains(httpResponse.statusCode) else {
            throw QNAPError.uploadFailed("WebDAV upload failed")
        }

        progress(1.0)
    }

    private func uploadViaMountPoint(fileURL: URL, remotePath: String, connection: QNAPConnection, progress: @escaping (Double) -> Void) async throws {
        let mountPath = "/Volumes/\(connection.name)\(remotePath)"
        let destPath = "\(mountPath)/\(fileURL.lastPathComponent)"

        progress(0.2)

        try FileManager.default.copyItem(at: fileURL, to: URL(fileURLWithPath: destPath))

        progress(1.0)
    }

    /// Create a directory on the NAS
    func createDirectory(path: String, name: String) async throws {
        guard let connection = activeConnection, let sid = sessionId else {
            throw QNAPError.notConnected
        }

        if connection.qnapProtocol == .api {
            let baseURL = buildBaseURL(for: connection)
            let encodedPath = path.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? path

            guard let url = URL(string: "\(baseURL)/cgi-bin/filemanager/utilRequest.cgi?func=createdir&sid=\(sid)&dest_path=\(encodedPath)&dest_folder=\(name)") else {
                throw QNAPError.invalidURL
            }

            let (_, response) = try await session.data(from: url)

            guard let httpResponse = response as? HTTPURLResponse, httpResponse.statusCode == 200 else {
                throw QNAPError.operationFailed("Failed to create directory")
            }
        } else if connection.qnapProtocol == .webdav {
            let scheme = connection.useSSL ? "https" : "http"
            let urlString = "\(scheme)://\(connection.host):\(connection.port)\(path)/\(name)/"

            guard let url = URL(string: urlString) else {
                throw QNAPError.invalidURL
            }

            var request = URLRequest(url: url)
            request.httpMethod = "MKCOL"

            let credentials = "\(connection.username):\(connection.password)"
            if let credData = credentials.data(using: .utf8) {
                request.setValue("Basic \(credData.base64EncodedString())", forHTTPHeaderField: "Authorization")
            }

            let (_, response) = try await session.data(for: request)

            guard let httpResponse = response as? HTTPURLResponse,
                  (200...299).contains(httpResponse.statusCode) else {
                throw QNAPError.operationFailed("Failed to create directory via WebDAV")
            }
        } else {
            let mountPath = "/Volumes/\(connection.name)\(path)/\(name)"
            try FileManager.default.createDirectory(atPath: mountPath, withIntermediateDirectories: true)
        }
    }

    // MARK: - Helper Methods

    private func buildBaseURL(for connection: QNAPConnection) -> String {
        let scheme = connection.useSSL ? "https" : "http"
        return "\(scheme)://\(connection.host):\(connection.port)"
    }

    private func startSessionKeepAlive() {
        // Keep session alive every 5 minutes
        sessionExpiryTimer = Timer.scheduledTimer(withTimeInterval: 300, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                await self?.refreshSession()
            }
        }
    }

    private func refreshSession() async {
        guard let connection = activeConnection, let sid = sessionId else { return }

        if connection.qnapProtocol == .api {
            let baseURL = buildBaseURL(for: connection)
            guard let url = URL(string: "\(baseURL)/cgi-bin/filemanager/utilRequest.cgi?func=get_extract_list&sid=\(sid)") else { return }

            _ = try? await session.data(from: url)
        }
    }
}

// MARK: - Supporting Types

/// File information from QNAP NAS
struct QNAPFileInfo: Identifiable, Hashable {
    var id: String { path }
    let name: String
    let path: String
    let isDirectory: Bool
    let size: Int64
    let modifiedDate: Date
    let owner: String

    var formattedSize: String {
        ByteCountFormatter.string(fromByteCount: size, countStyle: .file)
    }

    var icon: String {
        if isDirectory {
            return "folder.fill"
        }

        let ext = (name as NSString).pathExtension.lowercased()
        switch ext {
        case "swift", "rs", "py", "js", "ts", "go", "java", "c", "cpp", "h":
            return "doc.text.fill"
        case "md", "txt", "json", "yaml", "yml", "xml":
            return "doc.plaintext.fill"
        case "png", "jpg", "jpeg", "gif", "svg", "webp":
            return "photo.fill"
        case "mp4", "mov", "avi", "mkv":
            return "film.fill"
        case "zip", "tar", "gz", "rar", "7z":
            return "doc.zipper"
        default:
            return "doc.fill"
        }
    }
}

/// Errors from QNAP operations
enum QNAPError: LocalizedError {
    case invalidURL
    case invalidResponse
    case notConnected
    case authenticationFailed(String)
    case connectionFailed(String)
    case uploadFailed(String)
    case downloadFailed(String)
    case operationFailed(String)
    case fileNotFound(String)
    case pathNotFound(String)

    var errorDescription: String? {
        switch self {
        case .invalidURL:
            return "Invalid URL configuration"
        case .invalidResponse:
            return "Invalid response from NAS"
        case .notConnected:
            return "Not connected to NAS"
        case .authenticationFailed(let msg):
            return "Authentication failed: \(msg)"
        case .connectionFailed(let msg):
            return "Connection failed: \(msg)"
        case .uploadFailed(let msg):
            return "Upload failed: \(msg)"
        case .downloadFailed(let msg):
            return "Download failed: \(msg)"
        case .operationFailed(let msg):
            return "Operation failed: \(msg)"
        case .fileNotFound(let path):
            return "File not found: \(path)"
        case .pathNotFound(let path):
            return "Path not found: \(path)"
        }
    }
}
