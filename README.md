# Window2Clear

一个轻量级的 Windows 桌面工具，用于控制窗口透明度、居中、抖动与鼠标穿透。

基于 Win32 API 实现，无需安装，打开即用。仓库包含三个并行版本：

| 版本 | 语言 | 定位 | 体积 |
|------|------|------|------|
| **Window2Clear** | C++ | 原版完整功能（透明度 / 居中 / 抖动） | ~117 KB |
| **Windnut2Clear** | C++ | 增强版（透明即穿透 + 窗口置顶） | ~120 KB |
| **Window2Clear-rs** | Rust | 精简版（仅 Alt+←/→ 透明度，修复 bug） | ~260 KB |

> 本仓库基于 [iwill123/Window2Clear](https://github.com/iwill123/Window2Clear) 开源代码二次开发。

## 下载

无需编译，下载即用：

👉 **[前往 Release 下载最新版](https://github.com/sky12378/Window2Clear/releases)**

## 功能特性

### Window2Clear (C++ 原版, v0.3.0)

- **透明度调整**：1%–50% 步长可调，热键实时控制
- **透明度持久化**：10ms 定时器守护，避免窗口移动鼠标后透明状态丢失
- **一键恢复**：`Alt+↑` 立即恢复窗口不透明
- **窗口居中**：一键将活动窗口移至屏幕中心
- **窗口抖动**：趣味抖动动画
- **系统托盘**：最小化常驻托盘，右键菜单访问设置
- **热键全自定义**：所有热键均可在设置界面重新绑定

### Windnut2Clear (C++ 增强版)

在原版基础上新增：

| 特性 | 原版 | Windnut2Clear |
|------|------|---------------|
| 鼠标穿透 | ❌ 仅视觉透明 | ✅ 点击可透过窗口 |
| 穿透后操作 | — | ✅ 按住 Alt 临时恢复交互 |
| 窗口置顶 | ❌ | ✅ 一键置顶/取消 |
| 配置文件 | `config.ini` | `Windnut2Clear_config.ini`（独立） |

### Window2Clear-rs (Rust 精简版, v3-rust)

仅保留核心透明度调整功能，精简约 30% 代码，强化守护机制：

- 仅 `Alt+←` / `Alt+→` 两个热键
- timer 守护改为**无条件重设 layered + alpha**，根治鼠标 hover/重绘导致透明度消失
- 全局状态由 `static mut` 重构为 `thread_local! + RefCell<State>`，消除 `static_mut_refs` 警告
- 配置文件路径基于 exe 目录，修复从托盘/自启动时 cwd=System32 导致配置丢失
- 旧版配置键 `EnableTransparencyUp/Down` 自动迁移至新合并键 `EnableTransparency`
- 禁用总开关时遍历已透明窗口恢复 255，与"总开关"语义一致

## 默认热键

### Window2Clear / Windnut2Clear

| 功能 | 默认热键 | 备注 |
|------|----------|------|
| 增加透明度 | `Alt + ←` | 步长可在设置中调整 |
| 减少透明度 | `Alt + →` | — |
| 恢复不透明 | `Alt + ↑` | v0.3.0 新增 |
| 窗口居中 | `Ctrl + Num5` | 需先在设置中启用 |
| 窗口抖动 | `Alt + ↓` | 需先在设置中启用 |
| 打开设置 | 右键托盘图标 | — |

### Window2Clear-rs

| 功能 | 默认热键 |
|------|----------|
| 增加透明度（更透明） | `Alt + ←` |
| 减少透明度（更不透明） | `Alt + →` |
| 打开设置 | 右键托盘图标 |

> 热键冲突时，右键托盘 → 设置 → 点击对应输入框 → 按下新组合键即可重新绑定。

## 使用说明

1. **启动**：运行程序后自动最小化到系统托盘，弹出启动提示
2. **调整透明度**：选中目标窗口（使其成为活动窗口），按 `Alt + ←/→`
3. **恢复不透明**：原版按 `Alt + ↑`；Rust 精简版持续按 `Alt + →` 至 255
4. **设置**：右键托盘图标 → 设置，开启/关闭功能或修改热键，自动保存到 `config.ini`

## 编译

### 环境要求

- Windows 10/11
- C++ 版本：Visual Studio 2019/2022 或 MinGW-w64
- Rust 版本：Rust 1.70+ (edition 2021)

### 获取源码

```bash
git clone https://github.com/sky12378/Window2Clear.git
```

### C++ 版本 (Visual Studio)

1. 打开 `Window2Clear.sln`
2. 选择 Release 配置
3. 生成解决方案
4. 输出在 `x64/Release` 或 `Release` 目录

### C++ 版本 (MinGW)

```bash
cd Window2Clear/Window2Clear
windres -i Window2Clear.rc -o resource.o --include-dir=.
g++ -DUNICODE -D_UNICODE -o Window2Clear.exe Window2Clear.cpp resource.o \
    -luser32 -lshell32 -lcomctl32 -mwindows -O2
```

### Rust 版本

```bash
cd Window2Clear-rs
cargo build --release
# 产物：target/release/window2clear.exe
```

## 项目结构

```
Window2Clear/
├── Window2Clear/                # C++ 原版 (v0.3.0)
│   ├── Window2Clear.cpp         # 主程序 (Win32 API)
│   ├── Window2Clear.rc          # 资源文件
│   ├── Window2Clear.ico         # 图标
│   └── config.ini               # 配置示例
├── Windnut2Clear/               # C++ 增强版（鼠标穿透 + 置顶）
│   ├── Windnut2Clear/Windnut2Clear.cpp
│   └── Windnut2Clear_config.ini
├── Window2Clear-rs/              # Rust 精简版 (v3-rust)
│   ├── src/main.rs              # 主程序 (winapi crate)
│   └── Cargo.toml
├── Window2Clear.sln             # VS 解决方案
└── README.md
```

## 版本历史

### v3-rust (Rust 版)

- 重构：全局状态由 `static mut` 迁移至 `thread_local! + RefCell<State>`
- 修复：鼠标移动后透明度消失（timer 守护改为无条件重设 layered + alpha）
- 修复：`wide().as_ptr()` 悬垂指针
- 修复：`listening` 状态残留导致再次打开设置窗口首次按键失效
- 修复：禁用总开关时已透明窗口未恢复
- 新增：旧配置键自动迁移
- 精简：删除居中 / 抖动 / 恢复不透明功能，仅保留 Alt+←/→

### v0.3.0 (C++ 原版, 2026-06-29)

- 新增：透明度持久化守护（10ms 定时器）
- 新增：`Alt+↑` 恢复窗口不透明热键
- 新增：Rust 复刻版
- 修复：config.ini 相对路径导致配置丢失

### v0.2.0 (原版)

- 基础透明度控制（Alt+←/→）
- 窗口居中 / 抖动功能
- 系统托盘运行
- 热键自定义
- 配置文件保存

## 系统要求

- Windows 10/11

## 许可证

开源项目，欢迎贡献代码和反馈问题。

## 致谢

- 原项目作者：[iwill123](https://github.com/iwill123)
- 本修改版作者：[sky12378](https://github.com/sky12378)
