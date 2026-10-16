/**
 * Đưa đường dẫn chỉ biết lúc chạy (query `?next=`, href từ mảng menu) vào resolve().
 * Từ SvelteKit 2.63, resolve() chỉ nhận từng route literal nên kiểu hợp Pathname
 * không qua được kiểm kiểu. Hàm này chỉ đổi kiểu, giá trị giữ nguyên:
 * `resolve(dynamicRoute(path))` vẫn là base + path.
 */
export function dynamicRoute(path: string): '/' {
	return path as '/';
}
