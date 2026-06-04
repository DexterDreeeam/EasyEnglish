#include <gtest/gtest.h>

// Sanity check that the test harness compiles, links, and runs.
// Real per-module tests live under tests/unit/<module>/ from iter-001 onward.
TEST(Smoke, ArithmeticStillWorks) {
    EXPECT_EQ(1 + 1, 2);
}
