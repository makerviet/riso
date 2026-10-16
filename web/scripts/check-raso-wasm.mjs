// Chặn `vite build` sớm với thông báo rõ khi chưa build WASM bộ máy luật
// (`$raso` → ../crates/raso-rules/pkg). Chỉ kiểm tra tồn tại, không tự build:
// môi trường build web (CI, Cloudflare) có thể không có Rust/wasm-pack.
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const pkg = new URL('../../crates/raso-rules/pkg/', import.meta.url);
const missing = ['raso_rules.js', 'raso_rules_bg.wasm'].filter((f) => !existsSync(new URL(f, pkg)));
if (missing.length) {
	console.error(
		`Thiếu WASM bộ máy luật (${missing.join(', ')}) trong ${fileURLToPath(pkg)}.\n` +
			'Chạy ./scripts/build-rules-wasm.sh từ thư mục gốc repo trước khi pnpm build.'
	);
	process.exit(1);
}
