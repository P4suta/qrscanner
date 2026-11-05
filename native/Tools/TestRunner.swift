import Foundation
import XCTest

@main
struct NativeTestRunner {
    @MainActor
    static func main() {
        let suite = XCTestSuite(name: "QRScanner native verification")
        let suites = [PayloadTests.defaultTestSuite, CoreBridgeTests.defaultTestSuite, AppModelTests.defaultTestSuite,
                      QRDecoderTests.defaultTestSuite, InterfaceTests.defaultTestSuite,
                      ResultPanelTests.defaultTestSuite, ScreenCaptureTests.defaultTestSuite,
                      PreferencesMigrationTests.defaultTestSuite, AppDelegateTests.defaultTestSuite]
        suites.forEach(suite.addTest)
        suite.run()
        guard suites.allSatisfy({ child in
            guard child.testCaseCount > 0, let result = child.testRun else { return false }
            return result.executionCount == child.testCaseCount && result.hasSucceeded
        }), let result = suite.testRun, result.executionCount == suite.testCaseCount, result.hasSucceeded else {
            exit(EXIT_FAILURE)
        }
    }
}
