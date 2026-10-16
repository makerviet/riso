# Đóng góp cho RASO

Cảm ơn bạn! Cách đóng góp có giá trị nhất là **thêm một đề thi** (game pack).

## Thêm đề thi mới

1. Sao chép `pkgs/games/fgc-2026/` thành `pkgs/games/<id>/` (ví dụ `fgc-2019`, `fanroc-2026`).
2. Sửa `manifest.json`: `id`, `name`, `version`, các giai đoạn (`phases`), vật phẩm (`objects`), vị trí đứng (`lobby.stations`) và mục tiêu ghi điểm (`scoring.targets`).
3. Dựng sân trong Blender, xuất `field.glb`; sinh `field.physics.json` và `field.semantics.json`
   bằng các script trong `pkgs/scripts/`. Tên vùng/điểm neo ghi theo mẫu `FIELDSEMANTICSDEF.txt`.
4. Viết luật tính điểm trong `rules.yaml` (xem mục dưới); `rules/*.rhai` chỉ dành cho luật đặc biệt của sân.
5. Thêm kịch bản kiểm thử vào `tests/scenarios/`.
6. Mở Pull Request, mô tả: tên giải, năm, link game manual chính thức, phần luật đã/chưa mô phỏng.

## Viết luật tính điểm (`rules.yaml`)

1. Tạo `pkgs/games/<id>/rules.yaml` theo `docs/superpowers/specs/2026-10-15-raso-rules-design.md` (mục 4). Mẫu: `pkgs/games/fgc-2026/rules.yaml`, `pkgs/games/fgc-2025/rules.yaml`.
2. Đề dựa trên đề khác: `extends: <id-đề-cha>` + khối `override:` chỉ ghi phần thay đổi.
3. Viết ít nhất 6 biên bản mẫu trong `tests/scoring/*.yaml`, phủ mọi dòng trong bảng điểm của game manual và các ranh giới.
4. Chạy `cd crates/raso-rules && cargo run --features cli --bin raso -- check ../../pkgs/games/<id>`.
5. Thêm đề vào `pkgs/games/index.json` để hiện trên trang `/referee`.
6. Muốn chơi mô phỏng: thêm `observe.yaml` ánh xạ từng ô biên bản sang phép đo (`count_objects`, `brace_state`, `not_simulated`).

## Sửa code

- Rust: `cargo fmt --all` và `cargo test --all-targets` trong `server/` và `crates/raso-rules/` phải sạch.
- Web cần WASM luật: chạy `./scripts/build-rules-wasm.sh` trước `pnpm vitest run`.
- Web/API: `pnpm run check` và `pnpm run build` phải qua.
- Mỗi PR một chủ đề; commit theo kiểu `feat(scope): ...`, `fix(scope): ...`.

## Ngôn ngữ

Tài liệu ưu tiên tiếng Việt, có tiếng Anh càng tốt. Giao diện thêm chuỗi vào `web/messages/{vi,en}.json`.
