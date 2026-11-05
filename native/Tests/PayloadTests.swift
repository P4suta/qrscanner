import XCTest

final class PayloadTests: XCTestCase {
    func testCompiledEnglishCatalogUsesSingularAndPluralCounts() throws {
        let path = try XCTUnwrap(CommandLine.arguments.last)
        let bundle = try XCTUnwrap(Bundle(path: path))
        XCTAssertEqual(AppStrings.resultCount(0, bundle: bundle), "0 QR codes found")
        XCTAssertEqual(AppStrings.resultCount(1, bundle: bundle), "1 QR code found")
        XCTAssertEqual(AppStrings.resultCount(2, bundle: bundle), "2 QR codes found")
    }

    func testWebURLsPreserveQueriesAndFragments() throws {
        let value = "https://example.org/forms?id=42&name=Alex#question-2"
        let payload = ScannedPayload(value)
        XCTAssertEqual(try XCTUnwrap(payload.webURL).absoluteString, value)
        XCTAssertEqual(payload.text, value)
        XCTAssertEqual(payload.host, "example.org")
    }

    func testNonWebOrMalformedPayloadsAreCopyOnly() {
        for value in ["hello", "", "javascript:alert(1)", "file:///etc/hosts", "mailto:a@example.org",
                      "https://", "https://exa mple.org", "https://example.org/\nnext", "https://user:pass@example.org"] {
            XCTAssertNil(ScannedPayload(value).webURL, value)
        }
    }

    func testUnicodeAndPlainTextRemainCopyable() {
        let text = "説明会へのご参加ありがとうございます。\n回答をお願いします。"
        XCTAssertEqual(ScannedPayload(text).text, text)
        XCTAssertNil(ScannedPayload(text).webURL)
    }

    func testDuplicatesPreserveFirstOccurrence() {
        let result = ScannedPayload.unique(["https://example.org/a", "hello", "https://example.org/a", "hello"])
        XCTAssertEqual(result.map(\.text), ["https://example.org/a", "hello"])
    }
}
