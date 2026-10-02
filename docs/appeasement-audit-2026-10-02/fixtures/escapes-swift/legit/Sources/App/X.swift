import Foundation

func load(_ url: URL) throws -> Data {
    return try Data(contentsOf: url)
}
