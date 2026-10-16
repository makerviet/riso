# Chạy RASO tại máy

Đã kiểm chứng ngày 15/10/2026 trên macOS (Apple Silicon) và Ubuntu 24.04 (x86_64).

Yêu cầu: Node 24+, pnpm, Rust (rustup, bản stable), `wasm-pack` (`cargo install wasm-pack --locked`) và target `wasm32-unknown-unknown`.

## 1. API (cổng 8787)

```sh
cd api
pnpm install
cat > .dev.vars <<EOF
JWT_SECRET=$(openssl rand -hex 32)
WEB_ORIGIN=http://localhost:5173
API_ORIGIN=http://localhost:8787
EOF
pnpm wrangler d1 migrations apply fgsim --local --config wrangler.jsonc
pnpm dev
```

`WEB_ORIGIN`/`API_ORIGIN` phải ghi đè vì `wrangler.jsonc` mặc định trỏ domain production.

## 2. Tạo tài khoản quản trị đầu tiên

Mã mời phải đúng **6 ký tự chữ/số**; đăng ký bắt buộc trường `team`.

```sh
pnpm wrangler d1 execute fgsim --local --config wrangler.jsonc \
  --command "INSERT INTO invitations (code, used, createdAt) VALUES ('LOCAL1', 0, $(date +%s));"

curl -X POST http://localhost:8787/api/auth/sign-up/email \
  -H 'Content-Type: application/json' -H 'Origin: http://localhost:5173' \
  -d '{"email":"admin@local.test","password":"<mật-khẩu>","name":"Admin","team":"Vietnam","invitationCode":"LOCAL1"}'

pnpm wrangler d1 execute fgsim --local --config wrangler.jsonc \
  --command "UPDATE user SET role='admin' WHERE email='admin@local.test';"
```

## 3. Web (cổng 5173)

```sh
./scripts/build-rules-wasm.sh   # build WASM bộ máy luật (chạy từ thư mục gốc repo)
cd web && pnpm install && pnpm dev
```

Trang chấm điểm trận thật: http://localhost:5173/referee

Build bản production: luôn chạy `./scripts/build-rules-wasm.sh` (từ thư mục gốc) **trước** `pnpm build`
— web nhập bộ máy luật từ `crates/raso-rules/pkg`; thiếu gói này thì `pnpm build` dừng ngay và báo lỗi.
Sau mỗi lần sửa Rust trong `crates/raso-rules` cũng phải build lại WASM.

```sh
./scripts/build-rules-wasm.sh
cd web && pnpm build
```

## 4. Máy chủ game (cổng 3000)

Đăng nhập, rồi tạo khoá máy chủ game (trang Admin → Game servers, hoặc `POST /api/admin/game-servers`
với `{"origin":"http://localhost:3000"}`). Sao chép khoá một lần duy nhất:

```sh
cd server
cargo build --release
API_URL=http://localhost:8787 GAME_SERVER_KEY=<khoá> ./target/release/server
```

Máy chủ game giờ **bắt buộc** có `rules.yaml` và `observe.yaml` của đề trong gói runtime (snapshot do API phục vụ); thiếu thì không mở trận.

Mở http://localhost:5173 → **Open arena**.

## Kiểm thử & đo hiệu năng

```sh
(cd crates/raso-rules && cargo test --all-targets)
cd server
cargo test --release
cargo test --release benchmark_1000_ball_robot_interaction -- --ignored --nocapture
```

## Lỗi đã biết

- Nút **Practice** báo "A persisted match ID is required" — dùng Open arena hoặc tạo trận.
