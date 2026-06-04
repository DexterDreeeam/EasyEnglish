# Architecture Overview

> 详细版会在 iter-000 收尾时补充。这里只列模块依赖关系。

```
+-----------------+      +-----------------+      +------------------+
|    src/ui/      | ---> |    src/app/     | ---> |   src/core/      |
|  (Qt Widgets)   |      |  (orchestration)|      |  (pure logic)    |
+-----------------+      +-----------------+      +------------------+
        |                                                  |
        v                                                  v
   Qt6::Widgets                                       Qt6::Core only
                                                  +-------------------+
                                                  | dictionary        |
                                                  | storage  (sqlite) |
                                                  | history           |
                                                  | favorites         |
                                                  | network (mock-able)|
                                                  +-------------------+
```

强制纪律：

- `src/core/**` **不得** include `QtWidgets / QtGui / QtQuick`。
  由 `tools/check_core_no_ui.py` 在 CI 拦截。
- `src/ui/**` **不得** 直接 include `src/core/**` 实现细节，必须经 `src/app/**`
  这一层暴露的服务接口（iter-003 起强制）。
- 模块之间的"公共面"在 `docs/contracts/<module>.md` 里冻结。
