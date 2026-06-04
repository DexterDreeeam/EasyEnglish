#include <filesystem>

#include <benchmark/benchmark.h>

#include "core/dictionary/SqliteDictionary.hpp"
#include "core/storage/Database.hpp"

#ifndef EASYENGLISH_FIXTURES_DIR
#error "EASYENGLISH_FIXTURES_DIR must be defined by the build system"
#endif

namespace fs = std::filesystem;
using easyenglish::core::dictionary::SqliteDictionary;
using easyenglish::core::storage::Database;

namespace {

SqliteDictionary openMiniDict() {
    auto db = Database::open(fs::path(EASYENGLISH_FIXTURES_DIR) / "mini_dict.sqlite");
    if (!db.has_value()) {
        std::abort();
    }
    auto dict = SqliteDictionary::open(std::move(db.value()));
    if (!dict.has_value()) {
        std::abort();
    }
    return std::move(dict.value());
}

}  // namespace

static void BM_DictionaryLookupHit(benchmark::State& state) {
    auto dict = openMiniDict();
    for (auto _ : state) {
        auto entry = dict.lookup("apple");
        benchmark::DoNotOptimize(entry);
    }
}
BENCHMARK(BM_DictionaryLookupHit);

static void BM_DictionaryLookupMiss(benchmark::State& state) {
    auto dict = openMiniDict();
    for (auto _ : state) {
        auto entry = dict.lookup("nosuchword");
        benchmark::DoNotOptimize(entry);
    }
}
BENCHMARK(BM_DictionaryLookupMiss);
