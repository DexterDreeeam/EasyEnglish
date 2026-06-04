#include <utility>

#include <gtest/gtest.h>

#include "core/storage/Database.hpp"
#include "core/storage/Statement.hpp"

using easyenglish::core::storage::Database;
using easyenglish::core::storage::Statement;
using easyenglish::core::storage::StorageError;

namespace {

Database openSeeded() {
    auto db = Database::open(std::string(Database::kInMemory));
    EXPECT_TRUE(db.has_value());
    auto& d = db.value();
    EXPECT_TRUE(
        d.execute("CREATE TABLE words(headword TEXT PRIMARY KEY, count INTEGER);").has_value());
    EXPECT_TRUE(d.execute("INSERT INTO words VALUES('apple', 10),('banana', 5);").has_value());
    return std::move(db.value());
}

}  // namespace

TEST(Statement, BindAndStepText) {
    auto db = openSeeded();
    auto stmt_or = db.prepare("SELECT count FROM words WHERE headword = ?1;");
    ASSERT_TRUE(stmt_or.has_value());
    auto& stmt = stmt_or.value();

    ASSERT_TRUE(stmt.bind(1, std::string_view("apple")).has_value());
    auto row = stmt.step();
    ASSERT_TRUE(row.has_value());
    ASSERT_TRUE(row.value());
    EXPECT_EQ(stmt.columnInt64(0), 10);
}

TEST(Statement, ResetAndRebindForReuse) {
    auto db = openSeeded();
    auto stmt_or = db.prepare("SELECT count FROM words WHERE headword = ?1;");
    ASSERT_TRUE(stmt_or.has_value());
    auto& stmt = stmt_or.value();

    ASSERT_TRUE(stmt.bind(1, std::string_view("apple")).has_value());
    ASSERT_TRUE(stmt.step().value());
    EXPECT_EQ(stmt.columnInt64(0), 10);

    ASSERT_TRUE(stmt.reset().has_value());
    ASSERT_TRUE(stmt.bind(1, std::string_view("banana")).has_value());
    ASSERT_TRUE(stmt.step().value());
    EXPECT_EQ(stmt.columnInt64(0), 5);
}

TEST(Statement, NullValueIsDetected) {
    auto db = openSeeded();
    ASSERT_TRUE(db.execute("INSERT INTO words VALUES('null_word', NULL);").has_value());
    auto stmt_or = db.prepare("SELECT count FROM words WHERE headword = 'null_word';");
    ASSERT_TRUE(stmt_or.has_value());
    auto& stmt = stmt_or.value();

    ASSERT_TRUE(stmt.step().value());
    EXPECT_TRUE(stmt.isColumnNull(0));
}

TEST(Statement, MoveTransfersOwnership) {
    auto db = openSeeded();
    auto stmt_or = db.prepare("SELECT 1;");
    ASSERT_TRUE(stmt_or.has_value());

    Statement moved(std::move(stmt_or.value()));
    auto row = moved.step();
    ASSERT_TRUE(row.has_value());
    EXPECT_TRUE(row.value());
    EXPECT_EQ(moved.columnInt64(0), 1);
}

TEST(Statement, BindInt64AndDouble) {
    auto db = openSeeded();
    ASSERT_TRUE(db.execute("CREATE TABLE nums(i INTEGER, d REAL);").has_value());

    {
        auto stmt_or = db.prepare("INSERT INTO nums VALUES(?1, ?2);");
        ASSERT_TRUE(stmt_or.has_value());
        auto& stmt = stmt_or.value();
        ASSERT_TRUE(stmt.bind(1, std::int64_t{42}).has_value());
        ASSERT_TRUE(stmt.bind(2, 3.14).has_value());
        ASSERT_TRUE(stmt.step().has_value());
    }

    auto stmt_or = db.prepare("SELECT i, d FROM nums;");
    ASSERT_TRUE(stmt_or.has_value());
    auto& stmt = stmt_or.value();
    ASSERT_TRUE(stmt.step().value());
    EXPECT_EQ(stmt.columnInt64(0), 42);
    EXPECT_DOUBLE_EQ(stmt.columnDouble(1), 3.14);
}
