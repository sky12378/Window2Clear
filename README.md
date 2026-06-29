# Window2Clear

一个轻量级的 Windows 桌面工具，用于控制窗口透明度、居中和抖动效果。

## 点击链接直接下载，无需安装开箱即用

[下载 Window2Clear v0.3.0](https://github.com/sky12378/Window2Clear/releases/download/v0.3.0/Window2Clear.exe)

## 功能特性

- 🔄 **透明度控制**: 通过热键快速调整任意窗口的透明度
- 🪟 **透明度持久化**: 设置透明后自动守护，窗口不会意外恢复不透明
- ⬆️ **一键恢复**: `Alt+↑` 立即恢复窗口不透明
- 🎯 **窗口居中**: 一键将当前窗口移动到屏幕中央
- 🎭 **窗口抖动**: 为窗口添加抖动动画效果
- ⚙️ **热键自定义**: 支持自定义热键组合
- 💾 **配置保存**: 自动保存用户设置到 `config.ini`
- 🖼️ **系统托盘**: 最小化到系统托盘运行

## 默认热键

| 热键 | 功能 |
|------|------|
| `Alt + ←` | 增加透明度 |
| `Alt + →` | 减少透明度 |
| `Alt + ↑` | 恢复不透明 |
| `Ctrl + 数字键5` | 窗口居中（需在设置中启用） |
| `Alt + ↓` | 窗口抖动（需在设置中启用） |

## 使用说明

1. 运行程序后，会自动最小化到系统托盘
2. 使用热键对当前活动窗口进行操作
3. 右键点击托盘图标可以打开设置界面
4. 在设置界面中可以自定义热键和功能开关

## 双版本

本项目包含两个版本：

| 版本 | 语言 | 大小 | 说明 |
|------|------|------|------|
| `Window2Clear/` | C (Win32 API) | ~28KB | 原版，需 VS2022 或 MinGW 编译 |
| `Window2Clear-rs/` | Rust (winapi crate) | ~269KB | Rust 复刻版，需 Rust 工具链编译 |

## 编译说明

### C 版本 (Visual Studio)
1. 使用 Visual Studio 打开 `Window2Clear.sln`
2. 选择 Release 配置
3. 生成解决方案

### C 版本 (MinGW)
```bash
cd Window2Clear
windres -i Window2Clear.rc -o resource.o --include-dir=.
g++ -DUNICODE -D_UNICODE -o Window2Clear.exe Window2Clear.cpp resource.o -luser32 -lshell32 -lcomctl32 -mwindows -O2
```

### Rust 版本
```bash
cd Window2Clear-rs
cargo build --release
```

## 配置文件

程序设置保存在 `config.ini` 文件中，包含：
- 热键设置（每个功能的修饰键和按键）
- 功能开关（启用/禁用各功能）
- 透明度调整步长（1%-50%）

## 系统要求

- Windows 10/11

## 版本信息

当前版本: v0.3.0

## 许可证

开源项目，欢迎贡献代码和反馈问题。
