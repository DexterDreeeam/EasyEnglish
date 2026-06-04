# iter-005 Retrospective

- **What shipped**: `src/core/favorites/FavoritesStore` (idempotent add/remove, case-insensitive contains, list ordered by added_at desc, schema auto-create). MainWindow gets a star `QToolButton` + Favorites tab. 8 unit tests + 3 new UI tests.

- **AI走偏**: 重写 MainWindow 时把侧边栏从纯 QListWidget 改成 QTabWidget(history+favorites)。旧契约只列了 `historyList`，需要补一份"sideTabs / favoritesList"加进去；这次预先在 ui-mainwindow.md 把新 objectName 一起记录到 change log，避免 iter-006 接 fuzzy 时再 break。

- **下次保留**：iter-004 给 storage 加 `createOrOpen` 时形成的模式（用户数据 → createOrOpen；只读资源 → open）在 iter-005 自然复用，没出任何 IoError。这个二分应该写进 AGENTS.md 或 storage 契约的"使用指引"段。
