# iter-000 Retrospective

> 完成后填写。建议 1–3 行：
> - 我们做了什么（一句话）
> - AI 在哪里走偏过 / 出了什么意外
> - 下一轮要保留/改变什么习惯

- 在不到一个工作时段内立起脚手架：CMake/vcpkg/Qt 6.8.3/GTest/QTest/Google Benchmark + GitHub Actions CI 端到端绿，包含架构守卫 `check_core_no_ui.py`。

- AI 在三处偏差：(1) `vcpkg.json` 写了占位日期当 baseline，必须是真 SHA；(2) `lukka/run-vcpkg` 拒绝 `'master'`，需要完整 SHA1；(3) Qt 默认想走 vcpkg 源码编译（CI 上 1–3h），换 `install-qt-action` 走官方二进制后 7m20s 完成。三处都靠 CI 自动反馈在分钟级捕获，未污染 main 长期分支。

- 经验：今后任何"行号 / 列号 / 时间戳 / 版本号"占位字段都先用 ripgrep 把仓库里所有候选位置过一遍再 commit，否则 CI 浪费一次反馈周期。下一轮迭代（iter-001-storage）前先确认本地装好 Qt 6.8.3 + 一个本地 vcpkg，避免每次都依赖 CI 验证。
