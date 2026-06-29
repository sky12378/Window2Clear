# Window2Clear

一个轻量级的 Windows 桌面工具，用于控制窗口透明度、居中和抖动效果。

调用 Windows API 实现，超小体积，仅 28KB，无需安装，打开即用。

本项目基于 [iwill123/Window2Clear](https://github.com/iwill123/Window2Clear) 开源代码修改，新增透明度持久化守护、恢复热键及 Rust 复刻版。

## 下载与安装

### 直接下载 (推荐)

无需编译，下载即可运行：

👉 **[点击右侧 Release 下载最新版 Window2Clear v0.3.0](https://github.com/sky12378/Window2Clear/releases)**

提供两个版本：
- `Window2Clear.exe` - C 版本 (117KB)
- `Window2Clear-rs.exe` - Rust 版本 (269KB)

### 手动编译

如果您想自行修改源码：

**环境要求:** Windows 10/11, Visual Studio 2019/2022 或 MinGW-w64

**获取源码:**
```bash
git clone https://github.com/sky12378/Window2Clear.git
```

**编译步骤 (Visual Studio):**
1. 使用 VS 打开 `Window2Clear.sln`
2. 配置选择 Release
3. 生成解决方案
4. 生成的 .exe 文件在 `x64/Release` 或 `Release` 目录下

**编译步骤 (MinGW):**
```bash
cd Window2Clear/Window2Clear
windres -i Window2Clear.rc -o resource.o --include-dir=.
g++ -DUNICODE -D_UNICODE -o Window2Clear.exe Window2Clear.cpp resource.o -luser32 -lshell32 -lcomctl32 -mwindows -O2
```

**编译步骤 (Rust):**
```bash
cd Window2Clear-rs
cargo build --release
```

## 功能特性

### 1. 透明度控制

- **调整透明度:** 通过热键快速调整任意窗口的透明度 (1%-50% 步长可调)
- **透明度持久化:** 设置透明后自动守护，窗口不会意外恢复不透明
- **一键恢复:** `Alt+↑` 立即恢复窗口不透明状态

### 2. 窗口居中

- 一键将当前活动窗口移至屏幕中心
- 默认关闭，需在设置中启用

### 3. 窗口抖动

- 趣味抖动动画效果
- 默认关闭，需在设置中启用

### 4. 系统托盘

- 最小化后常驻系统托盘
- 右键菜单快速访问设置

### 5. 热键全自定义

- 所有功能热键均可在设置界面重新设置
- 点击对应输入框按下新组合键即可修改

## 默认热键

| 功能 | 默认热键 | 备注 |
|------|----------|------|
| 增加透明度 | `Alt + ←` (左) | 步长可在设置中调整 |
| 减少透明度 | `Alt + →` (右) | - |
| 恢复不透明 | `Alt + ↑` (上) | v0.3.0 新增 |
| 窗口居中 | `Ctrl + Num5` (小键盘5) | 需先在设置中启用 |
| 窗口抖动 | `Alt + ↓` (下) | 需先在设置中启用 |
| 打开设置界面 | 右键点击托盘图标 | - |

> 提示: 如果默认热键与其他软件冲突，请在设置界面点击对应输入框并按下您想要的新组合键即可修改。

## 使用说明

1. **启动:** 运行程序后，它会自动最小化到系统托盘（右下角），并弹出启动提示。
2. **调整透明度:** 选中任意窗口（使其成为活动窗口），使用 `Alt + ←/→` 调整透明度。
3. **恢复透明度:** 按 `Alt + ↑` 将窗口恢复为完全不透明。
4. **设置:** 右键托盘图标 → 设置，在此处开启/关闭功能或修改热键。修改后自动保存到 `config.ini`。

## 项目结构

```
Window2Clear/
├── Window2Clear/          # C 版本源码
│   ├── Window2Clear.cpp   # 主程序 (Win32 API)
│   ├── Window2Clear.rc    # 资源文件
│   ├── resource.h         # 资源头文件
│   ├── Window2Clear.ico   # 图标
│   └── config.ini         # 配置文件
├── Window2Clear-rs/       # Rust 版本源码
│   ├── src/main.rs        # 主程序 (winapi crate)
│   └── Cargo.toml         # Rust 项目配置
├── Window2Clear.sln       # VS 解决方案
└── README.md              # 本文件
```

## 版本历史

### v0.3.0 (2026-06-29)

- 新增: 透明度持久化守护 (10ms 定时器，自动恢复丢失的透明状态)
- 新增: `Alt+↑` 恢复窗口不透明热键
- 新增: Rust 复刻版 (Window2Clear-rs)
- 新增: .gitignore
- 修复: 透明窗口移动鼠标后消失问题
- 修复: config.ini 相对路径导致配置丢失问题

### v0.2.0 (原版)

- 基础透明度控制 (Alt+←/→)
- 窗口居中功能
- 窗口抖动功能
- 系统托盘运行
- 热键自定义
- 配置文件保存

## 系统要求

- Windows 10/11

## 许可证

开源项目，欢迎贡献代码和反馈问题。

## 致谢

本项目基于 [iwill123/Window2Clear](https://github.com/iwill123/Window2Clear) 开源代码修改。

- 原项目作者: [iwill123](https://github.com/iwill123)
- 本修改版作者: [sky12378](https://github.com/sky12378)
