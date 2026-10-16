import { defineConfig } from 'vitest/config';

export default defineConfig({
	resolve: { alias: { $raso: new URL('../crates/raso-rules/pkg', import.meta.url).pathname } },
	test: { include: ['src/**/*.test.ts'], environment: 'node' }
});
