import { defineConfig } from 'vitest/config';

export default defineConfig({
	resolve: { alias: { $riso: new URL('../crates/riso-rules/pkg', import.meta.url).pathname } },
	test: { include: ['src/**/*.test.ts'], environment: 'node' }
});
