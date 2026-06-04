#include <filesystem>

#include <gtest/gtest.h>

#include "core/storage/Database.hpp"

namespace fs = std::filesystem;
using easyenglish::core::storage::Database;
using easyenglish::core::storage::StorageError;

namespace {

Database openInMemory() {
    auto db = Database::open(std::string(Database::kInMemory));
    EXPECT_TRUE(db.has_value()) << "open(:memory:) failed";
    return std::move(db.value());
}

}  // namespace

TEST(DatabaseOpen, InMemorySucceeds) {
    auto db = Database::open(std::string(Database::kInMemory));
    ASSERT_TRUE(db.has_value());
}

TEST(DatabaseOpen, MissingFileReturnsIoError) {
    const fs::path missing = fs::temp_directory_path() / "easyenglish_does_not_exist.sqlite";
    if (fs::exists(missing)) {
        fs::remove(missing);
    }
    auto db = Database::open(missing);
    ASSERT_FALSE(db.has_value());
    EXPECT_EQ(db.error(), StorageError::IoError);
}

TEST(DatabaseExecute, RoundTripCreateInsertSelect) {
    auto db = openInMemory();
    ASSERT_TRUE(
        db.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, name TEXT NOT NULL);").has_value());
    ASSERT_TRUE(db.execute("INSERT INTO t(name) VALUES('alpha'),('beta');").has_value());

    auto stmt_or = db.prepare("SELECT id, name FROM t ORDER BY id;");
    ASSERT_TRUE(stmt_or.has_value());
    auto& stmt = stmt_or.value();

    auto step1 = stmt.step();
    ASSERT_TRUE(step1.has_value());
    ASSERT_TRUE(step1.value());
    EXPECT_EQ(stmt.columnInt64(0), 1);
    EXPECT_EQ(stmt.columnText(1), "alpha");

    auto step2 = stmt.step();
    ASSERT_TRUE(step2.has_value());
    ASSERT_TRUE(step2.value());
    EXPECT_EQ(stmt.columnInt64(0), 2);
    EXPECT_EQ(stmt.columnText(1), "beta");

    auto step3 = stmt.step();
    ASSERT_TRUE(step3.has_value());
    EXPECT_FALSE(step3.value());  // DONE
}

TEST(DatabaseExecute, InvalidSqlReturnsInvalidQuery) {
    auto db = openInMemory();
    auto result = db.execute("NOT VALID SQL;");
    ASSERT_FALSE(result.has_value());
    EXPECT_EQ(result.error(), StorageError::InvalidQuery);
}

TEST(DatabaseExecute, UniqueViolationReturnsConstraintViolation) {
    auto db = openInMemory();
    ASSERT_TRUE(db.execute("CREATE TABLE t(name TEXT PRIMARY KEY);").has_value());
    ASSERT_TRUE(db.execute("INSERT INTO t VALUES('only');").has_value());

    auto result = db.execute("INSERT INTO t VALUES('only');");
    ASSERT_FALSE(result.has_value());
    EXPECT_EQ(result.error(), StorageError::ConstraintViolation);
}

TEST(DatabasePrepare, InvalidSqlReturnsInvalidQuery) {
    auto db = openInMemory();
    auto stmt = db.prepare("SELECT FROM WHERE;");
    ASSERT_FALSE(stmt.has_value());
    EXPECT_EQ(stmt.error(), StorageError::InvalidQuery);
}

TEST(DatabaseMove, MoveConstructorTransfersOwnership) {
    auto db1 = openInMemory();
    ASSERT_TRUE(db1.execute("CREATE TABLE t(x INTEGER);").has_value());

    Database db2(std::move(db1));
    // After move, db2 owns the connection. Use it.
    auto result = db2.execute("INSERT INTO t VALUES(42);");
    EXPECT_TRUE(result.has_value());
}

TEST(DatabaseMove, MoveAssignmentClosesOldConnection) {
    auto db1 = openInMemory();
    auto db2 = openInMemory();
    ASSERT_TRUE(db1.execute("CREATE TABLE a(x INTEGER);").has_value());
    ASSERT_TRUE(db2.execute("CREATE TABLE b(y INTEGER);").has_value());

    db1 = std::move(db2);
    // After move-assign, db1's old connection is closed and it now points at
    // what was db2. Table 'b' should be reachable, 'a' should not.
    EXPECT_TRUE(db1.execute("SELECT y FROM b;").has_value());
    auto missing = db1.execute("SELECT x FROM a;");
    EXPECT_FALSE(missing.has_value());
}
