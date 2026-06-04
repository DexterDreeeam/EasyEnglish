# iter-008 Retrospective

- **What shipped**: 一个真 SqliteDictionary + 真 HistoryStore/FavoritesStore + MainWindow 的 E2E 测试（搜词 → 收藏 → 验证侧栏）；Inno Setup `.iss` + PowerShell 打包脚本；CI 增加 benchmark 烟雾运行。

- **AI走偏**: 起初想在 CI 上直接装 Inno Setup 跑安装包构建——意义不大（runner image 还要预热缓存）。改为只在 commit 里提供脚本，本地 dev 一行命令搞定，CI 仍只跑测试。

- **下次保留**：E2E test 不依赖任何 mock；模板就是先列出全部 widget 子对象 → 用 keyClicks/mouseClick 驱动 → 断言 UI/侧栏状态。把这套结构变成"端到端模板"加入 docs/prompts/ 会进一步降低后续 iter 的成本。
