import XCTest

@testable import HeadroomKit

final class ScrollIndicatorTests: XCTestCase {
    func testNoKnobWhenEverythingFits() {
        XCTAssertNil(ScrollIndicator.knob(viewport: 500, content: 500, offset: 0))
        XCTAssertNil(ScrollIndicator.knob(viewport: 500, content: 300, offset: 0))
        XCTAssertNil(ScrollIndicator.knob(viewport: 0, content: 300, offset: 0))
    }

    func testKnobLengthIsProportionalToTheVisibleShare() {
        let knob = ScrollIndicator.knob(viewport: 410, content: 820, offset: 0)
        XCTAssertEqual(knob, ScrollKnob(top: 5, length: 200))
    }

    func testKnobReachesTheBottomInsetAtTheEnd() {
        let knob = ScrollIndicator.knob(viewport: 410, content: 820, offset: 410)
        XCTAssertEqual(knob, ScrollKnob(top: 205, length: 200))
    }

    func testOverscrollIsClamped() {
        XCTAssertEqual(ScrollIndicator.knob(viewport: 410, content: 820, offset: -40)?.top, 5)
        XCTAssertEqual(ScrollIndicator.knob(viewport: 410, content: 820, offset: 900)?.top, 205)
    }

    func testVeryLongContentKeepsAMinimumKnob() {
        let knob = ScrollIndicator.knob(viewport: 110, content: 100_000, offset: 0)
        XCTAssertEqual(knob?.length, ScrollIndicator.minLength)
    }
}
