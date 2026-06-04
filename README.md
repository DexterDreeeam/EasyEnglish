# EasyEnglish

> AI 协作开发的 Windows 单词查询桌面应用 — C++23 / Qt 6 / CMake / vcpkg

本项目的特别之处在于：**全程由 AI 协助编写**。仓库里大量的"看似与功能无关"的文件
（`AGENTS.md`、`docs/contracts/`、`docs/adr/`、`docs/prompts/`、各种 CI 守卫脚本）
都是为了让 AI 在跨会话、跨模块、跨贡献者时仍然能产出一致、可验证、可回滚的代码。

## 如何参与（人类 / AI 都适用）

1. **先读** [`AGENTS.md`](./AGENTS.md) — 这是仓库的"宪法"。
2. **再读** [`docs/contracts/`](./docs/contracts/) 中你要改动的模块契约。
3. **写任务卡**：复制 [`docs/prompts/feature-task.md`](./docs/prompts/feature-task.md) 到
   `docs/iterations/iter-NNN-<slug>/task.md`，填好 acceptance criteria。
4. **AI 实现 + 自检**：按 `AGENTS.md` 末尾的 self-check 清单逐项回答。
5. **PR**：模板会要求你贴出本地测试结果与影响范围。

完整开发方法见 [实施计划（session 内 plan.md）](./docs/plan.md) — 该文件由用户在
session 中维护，不一定提交到仓库。

## 本地构建

前置：Visual Studio 2022（Desktop C++）+ vcpkg + Python 3.x。

```powershell
$env:VCPKG_ROOT = "<path-to-vcpkg>"
cmake --preset msvc-debug
cmake --build --preset msvc-debug --parallel
ctest --preset msvc-debug --output-on-failure
python tools\check_core_no_ui.py
```

## 顶层布局

```
src/core/   纯逻辑层（不依赖 QtWidgets/QtGui）
src/app/    编排层
src/ui/     Qt Widgets 视图层
tests/      unit + ui + benchmarks
docs/       contracts, adr, prompts, iterations
tools/      CI 守卫脚本与开发辅助
```

详细职责见 [`docs/architecture.md`](./docs/architecture.md)（iter-000 收尾时补上）。

## 许可证

MIT — 见 `LICENSE`（待补）。
