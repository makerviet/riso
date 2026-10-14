# Đóng góp cho RISO

Cảm ơn bạn! Cách đóng góp có giá trị nhất là **thêm một đề thi** (game pack).

## Thêm đề thi mới

1. Sao chép `pkgs/games/fgc-2026/` thành `pkgs/games/<id>/` (ví dụ `fgc-2019`, `fanroc-2026`).
2. Sửa `manifest.json`: `id`, `name`, `version`, các giai đoạn (`phases`), vật phẩm (`objects`), vị trí đứng (`lobby.stations`) và mục tiêu ghi điểm (`scoring.targets`).
3. Dựng sân trong Blender, xuất `field.glb`; sinh `field.physics.json` và `field.semantics.json`
   bằng các script trong `pkgs/scripts/`. Tên vùng/điểm neo ghi theo mẫu `FIELDSEMANTICSDEF.txt`.
4. Viết luật trong `rules/*.rhai`.
5. Thêm kịch bản kiểm thử vào `tests/scenarios/`.
6. Mở Pull Request, mô tả: tên giải, năm, link game manual chính thức, phần luật đã/chưa mô phỏng.

> **Lưu ý (10/2026):** hiện server mới gọi hook `on_trigger_enter`. Các hook
> `on_phase_start`, `on_match_end` và luật phạt chưa được nối — xem `docs/ROADMAP.md`.
> Ghi rõ trong PR nếu đề của bạn phụ thuộc vào chúng.

## Sửa code

- Rust: `cargo fmt --all` và `cargo test --all-targets` trong `server/` phải sạch.
- Web/API: `pnpm run check` và `pnpm run build` phải qua.
- Mỗi PR một chủ đề; commit theo kiểu `feat(scope): ...`, `fix(scope): ...`.

## Ngôn ngữ

Tài liệu ưu tiên tiếng Việt, có tiếng Anh càng tốt. Giao diện thêm chuỗi vào `web/messages/{vi,en}.json`.
