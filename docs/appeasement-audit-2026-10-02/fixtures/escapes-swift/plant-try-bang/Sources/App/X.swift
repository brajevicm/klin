import Foundation

func load(_ url: URL) -> Data {
    return try! Data(contentsOf: url)
}
