// SPDX-License-Identifier: MIT

import Foundation

final class APIClient {
    static let shared = APIClient()

    var baseURL: String {
        UserDefaults.standard.string(forKey: "serverURL") ?? "http://localhost:3000"
    }

    private let session: URLSession
    private let decoder: JSONDecoder

    private init() {
        session = URLSession.shared

        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss"
        formatter.locale = Locale(identifier: "en_US_POSIX")
        // Interpret naive datetimes as UTC
        formatter.timeZone = TimeZone(identifier: "UTC")

        decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .formatted(formatter)
    }

    // MARK: - Generic GET

    func get<T: Decodable>(_ path: String) async throws -> T {
        guard let url = URL(string: "\(baseURL)\(path)") else {
            throw APIError.invalidURL
        }

        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await session.data(from: url)
        } catch {
            throw APIError.networkError(error)
        }

        if let httpResponse = response as? HTTPURLResponse,
           !(200...299).contains(httpResponse.statusCode) {
            let body = String(data: data, encoding: .utf8) ?? "No response body"
            throw APIError.serverError(httpResponse.statusCode, body)
        }

        do {
            return try decoder.decode(T.self, from: data)
        } catch {
            throw APIError.decodingError(error)
        }
    }

    // MARK: - Buckets

    func fetchBuckets() async throws -> [BucketRow] {
        try await get("/api/buckets")
    }

    func fetchBucket(id: UUID) async throws -> BucketRow {
        try await get("/api/buckets/\(id)")
    }

    // MARK: - Commits

    func fetchCommits(bucketId: UUID) async throws -> [CommitRow] {
        try await get("/api/buckets/\(bucketId)/commits")
    }

    func fetchFiles(commitId: UUID) async throws -> [FileRow] {
        try await get("/api/commits/\(commitId)/files")
    }

    // MARK: - Expectations

    func fetchExpectations() async throws -> [ExpectationRow] {
        try await get("/api/expectations")
    }

    func fetchExpectationsForBucket(id: UUID) async throws -> [ExpectationRow] {
        try await get("/api/buckets/\(id)/expectations")
    }

    // MARK: - Pebbles

    func fetchPebbles() async throws -> [PebbleRow] {
        try await get("/api/pebbles")
    }

    func fetchPebblesForBucket(id: UUID) async throws -> [PebbleRow] {
        try await get("/api/buckets/\(id)/pebbles")
    }

    // MARK: - Graph

    func fetchGraph() async throws -> GraphData {
        try await get("/api/graph")
    }

    // MARK: - Search

    func search(query: String) async throws -> [SemanticResult] {
        let encoded = query.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? query
        return try await get("/api/search?q=\(encoded)")
    }
}
