# Architecture Overview

> Updated in iter-009 (Qt → Dear ImGui).

```
+-----------------+      +-----------------+      +------------------+
|    src/ui/      | ---> |    src/app/     | ---> |   src/core/      |
|  (Dear ImGui)   |      | AppState model  |      |  (pure logic)    |
+-----------------+      +-----------------+      +------------------+
        |                                                  |
        v                                                  v
   imgui::imgui +                                  +-------------------+
   glfw3 + OS GL                                   | dictionary        |
                                                   | storage  (sqlite) |
                                                   | history           |
                                                   | favorites         |
                                                   | network (httplib) |
                                                   +-------------------+
```

强制纪律：

- `src/core/**` **不得** include 任何 UI 库的头（`imgui`、`GLFW/`、`Qt*`）。
  由 `tools/check_core_no_ui.py` 在 CI 拦截。
- `src/ui/**` **不得** 直接 include `src/core/**` 实现细节，必须经 `src/app/**`
  这一层暴露的服务接口。MainView 只读写 AppState。
- 模块之间的"公共面"在 `docs/contracts/<module>.md` 里冻结。
