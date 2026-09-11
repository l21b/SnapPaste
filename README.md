# SnapPaste

简体中文 | [English](README.en.md)

基于 Rust + Slint 构建的轻量级剪贴板历史工具，支持快捷呼出、快速搜索和一键粘贴。

<p align="left">
  <img src="image.png" width="30%" alt="SnapPaste 界面预览一" />
  <img src="image-2.png" width="30%" alt="SnapPaste 界面预览二" />
  <img src="image-1.png" width="30%" alt="SnapPaste 界面预览三" />
</p>

## 功能特性

- **快捷呼出**：支持全局快捷键，默认 `Alt+Z`，窗口显示在鼠标附近。
- **历史与收藏**：分别查看剪贴板历史和收藏内容，支持搜索。
- **记录管理**：支持置顶、收藏、删除和清空记录。
- **收藏导入导出**：使用 JSON 文件备份、迁移收藏内容。
- **主题切换**：支持浅色、深色及跟随系统主题。

## 使用方式

1. 复制文本，SnapPaste 会自动记录剪贴板内容。
2. 在需要粘贴的位置按 `Alt+Z` 呼出窗口。
3. 搜索并点击所需记录，将内容粘贴到原来的窗口。
4. 将常用内容收藏或置顶，方便下次使用。

右键点击托盘图标可进入设置、查看关于页面或退出程序。在设置中可以修改快捷键、主题、记录保留天数、最大记录数和开机启动选项。

## 当前版本说明

当前版本以文本剪贴板功能为主，暂未开放图片记录与图片粘贴功能。

## 技术栈

- Rust
- Slint
- SQLite（rusqlite）
- arboard
- enigo
- clipboard-master
