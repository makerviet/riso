# Lộ trình RASO

## Hiện trạng trước giai đoạn 1 (đầu 10/2026)

- Vật lý và đồng bộ mạng rất tốt: bộ giải bóng XPBD riêng, giao thức nhị phân FGS1, p99 < 1 ms với 1.000 bóng.
- Tính điểm thật chỉ có cộng điểm từng bóng vào Suppression Unit, **viết cứng trong Rust**
  (`server/src/game/sphere_runtime/scoring.rs`).
- Chỉ hook Rhai `on_trigger_enter` được nối. `on_phase_start`, `on_match_end`, `penalties.rhai`,
  hệ số leo, Coopertition **chưa chạy**.
- Trận chỉ có TELEOP 150 s, chưa có AUTO/ENDGAME.
- Control plane khoá vào Cloudflare (Workers, D1, Durable Objects, R2) và Fly.io.

## Giai đoạn 1 — Bộ máy tính điểm kiểu FIRST — Xong (10/2026)

- Đặc tả điểm khai báo bằng file (JSON/YAML + JSON Schema): vật phẩm, vùng, giai đoạn
  AUTO/TELEOP/ENDGAME, hệ số nhân, lỗi phạt, Coopertition. Rhai chỉ cho luật đặc biệt.
- Gỡ phần điểm viết cứng khỏi Rust; nối đủ các hook.
- Bảng chi tiết điểm từng trận (score breakdown).

Đã làm: crate `raso-rules` (rules.yaml, biên bản, `raso check`, WASM), server tính điểm từ luật, trang `/referee`, CI job `rules`.

Còn tồn / ngoài phạm vi:

- Extinguisher và Partner climb chưa mô phỏng (chấm tay trên biên bản, `not_simulated`).
- SeaHeal chờ ban tổ chức công bố luật.
- Phần cộng điểm viết cứng trong Rust đã gỡ. Lỗi phạt giờ khai báo ở mục `fouls` của `rules.yaml` (trọng tài nhập số lần trên `/referee`); `penalties.rhai` đã xoá. Hook `on_phase_start`, `on_match_end` của Rhai vẫn chưa nối.
- `pnpm run check` ở `web/` còn 9 lỗi typed-href có sẵn từ upstream (layouts, trang auth) nên job CI `web` đỏ ở bước này; chưa sửa trong giai đoạn 1 (CI chạy `pnpm vitest run` trước bước này để test vẫn chạy).

### Còn lại của giai đoạn 1

- Trang trọng tài song ngữ vi/en: chuyển chuỗi giao diện sang Paraglide, hiển thị `label_en` của ô biên bản/giá trị/lỗi.
- Cột giá trị đã làm tròn trên bảng điểm (hiện chỉ hiện giá trị thô và điểm cuối).
- Hiện giá trị có nhãn cấp robot (`values.robot`) trên trang trọng tài và biên bản in.
- Việc nhỏ hoãn lại:
  - `override` gõ sai khoá thì tạo khoá mới thay vì báo lỗi.
  - CLI `raso check` panic ở một số ca `expect` (cần báo lỗi có mã thay vì panic).
  - Khoá phân biệt luật trong server dựa trên con trỏ `Arc` — đổi gói (cùng nội dung) cũng bị coi là luật mới.
  - API trả lỗi dạng chữ thô (plain text) thay vì JSON có mã lỗi.

## Giai đoạn 2 — Tầng quản lý giải

- Xếp lịch vòng loại với liên minh ngẫu nhiên (thể thức FAnRoC: mỗi đội 4 trận × 3 phút).
- Ranking Point, tiêu chí phân định khi bằng điểm, chọn liên minh, playoff.

## Giai đoạn 3 — Tự host, offline, lớp học

- Docker Compose: API Hono chạy trên Node + SQLite/Postgres, phòng chờ WebSocket trong tiến trình.
- Chế độ lớp học: giáo viên tạo lớp, học sinh vào bằng mã, không cần email; chế độ khách.

## Giai đoạn 4 — Chế độ chiến thuật 2D

- Sân nhìn từ trên, dùng chung luật và bộ tính điểm, vật lý đơn giản; chạy được trên máy tính trường.
- Học sinh lập kế hoạch đường đi/chu trình; máy chạy hàng loạt trận giả lập để ước điểm.
- Về sau: lập trình robot tự động bằng khối lệnh.

## Giai đoạn 5 — Thư viện đề

- Công cụ `raso pack new|validate|test`, repo đề riêng nhận PR.
- Đề FGC: 2017 H2O Flow, 2018 Energy Impact, 2019 Ocean Opportunities, 2022 Carbon Capture,
  2023 Hydrogen Horizons, 2024 Feeding the Future, 2025 Eco Equilibrium, 2026 Igniting Innovation.
  (Tên đề 2017/2018/2024 cần đối chiếu trang lưu trữ chính thức.)
- Đề FAnRoC theo thể lệ từng mùa.
