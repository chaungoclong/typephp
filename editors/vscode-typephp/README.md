# TypePHP VS Code Extension

Extension hỗ trợ đầy đủ **Syntax Highlighting** và **Language Server (LSP)** cho TypePHP (`.tphp`):
- **Hover**: xem kiểu dữ liệu, chữ ký hàm/class/type và số lượng references khi di chuột.
- **Go to Definition** (`Ctrl+Click` / `F12`): nhảy tới nơi khai báo biến, tham số, hàm, class, interface, enum, type alias.
- **Find All References** (`Shift+F12`): liệt kê tất cả vị trí tham chiếu.
- **Live Diagnostics**: hiển thị lỗi kiểu, đệ quy vô hạn, cảnh báo bộ nhớ trực tiếp trong editor.

## Cài đặt cục bộ

1. Cài đặt `tphpc` vào `PATH` (qua `install/install.sh` hoặc `cargo install --path .`).
2. Symlink extension vào VS Code (không cần `npm install`):

```bash
ln -s /home/long/Dev/typephp/editors/vscode-typephp ~/.vscode/extensions/typephp-language
```

3. Reload VS Code (`Developer: Reload Window`).
