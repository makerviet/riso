# RISO — Robot Is Sport

**Mô phỏng chiến thuật thi đấu robot cho học sinh** — đề FIRST Global Challenge các năm, FAnRoC (FPT Schools AI & Robotics Challenge) và các giải do cộng đồng đóng góp.

*A robot-competition strategy simulator for students: FIRST Global Challenge games across the years, FAnRoC, and community-made games.*

> Khởi tạo từ **Team Vietnam — FIRST Global Challenge 2026** (tác giả gốc: Khang Ngo-Bao, Tran Huu Duc Hieu),
> phát triển tiếp dưới dạng mã nguồn mở MIT bởi cộng đồng **MakerViet**.

## Có gì trong dự án

| Thư mục | Thành phần | Công nghệ |
|---|---|---|
| `server/` | Máy chủ trận đấu có thẩm quyền: vật lý, luật, tính điểm | Rust · axum (WebSocket) · rapier3d · bộ giải bóng XPBD · Rhai |
| `web/` | Giao diện chơi 3D, phòng chờ, quản trị | SvelteKit 5 · Threlte/three.js · Tailwind · Paraglide (vi/en/fr/de) |
| `api/` | Tài khoản, trận đấu, phòng chờ, điều phối máy chủ game | Hono · Drizzle · Cloudflare Workers/D1/Durable Objects |
| `pkgs/games/<id>/` | **Gói đề thi** (game pack): sân, vật phẩm, robot, luật | `manifest.json` + `field.glb` + JSON vật lý/vùng + `rules/*.rhai` |

Hiện có gói đề `fgc-2026` (*Igniting Innovation*).

Hiệu năng đo thật (1.000 bóng va chạm, ngân sách 16,67 ms/tick):
Apple Silicon p99 = 0,44 ms · Intel i5-12400F p99 = 0,75 ms.

## Chạy tại máy

Xem hướng dẫn từng bước: [`docs/LOCAL_SETUP.md`](docs/LOCAL_SETUP.md).

Tóm tắt: `api/` (cổng 8787) → `web/` (cổng 5173) → `server/` (cổng 3000, cần `API_URL` + `GAME_SERVER_KEY`).

## Lộ trình

Xem [`docs/ROADMAP.md`](docs/ROADMAP.md): bộ máy tính điểm kiểu FIRST khai báo bằng file, tầng quản lý giải (xếp lịch, ranking point, playoff), chế độ chiến thuật 2D, tự host offline cho trường học, thêm đề FGC 2017–2025 và FAnRoC.

## Đóng góp

Muốn thêm một đề thi mới? Xem [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Giấy phép & thương hiệu

Mã nguồn: [MIT](LICENSE). Xem [`NOTICE.md`](NOTICE.md) về nguồn gốc và thương hiệu.

RISO là dự án cộng đồng độc lập, **không phải sản phẩm chính thức và không được FIRST®, FIRST Global hay FPT Schools bảo trợ**. Tên các giải đấu chỉ dùng để mô tả đề thi được mô phỏng.
