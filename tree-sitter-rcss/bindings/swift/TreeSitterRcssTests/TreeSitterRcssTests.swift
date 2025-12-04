import XCTest
import SwiftTreeSitter
import TreeSitterRcss

final class TreeSitterRcssTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_rcss())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading RCSS grammar")
    }
}
