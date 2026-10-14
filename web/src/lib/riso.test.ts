import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { draftKey, loadRules } from './riso';

const games = join(__dirname, '../../../pkgs/games');
const wasm = () => readFileSync(join(__dirname, '../../../crates/riso-rules/pkg/riso_rules_bg.wasm'));
const fakeFetch: typeof fetch = async (input) => {
	const url = String(input);
	const id = url.split('/').pop()!;
	try {
		return new Response(readFileSync(join(games, id, 'rules.yaml'), 'utf8'), { status: 200 });
	} catch {
		return new Response('not found', { status: 404 });
	}
};

describe('loadRules', () => {
	it('tải đề và mô tả ô biên bản', async () => {
		const { rules } = await loadRules('fgc-2026', fakeFetch, wasm());
		expect(rules.game).toBe('fgc-2026');
		expect(Object.keys(rules.inputs.robot)).toEqual(['climb', 'lifted_by_partner']);
		expect(rules.sides.teams).toBe(3);
	});

	it('báo lỗi rõ khi đề không tồn tại', async () => {
		await expect(loadRules('khong-co', fakeFetch, wasm())).rejects.toThrow(/khong-co/);
	});

	it('khoá bản nháp theo đề', () => {
		expect(draftKey('fgc-2026')).toBe('riso-referee-draft:fgc-2026');
	});

	it('tải đề cha theo extends', async () => {
		const child =
			'schema: riso.rules/v1\nextends: fgc-2026\ngame: fgc-2026-mini\nname: Mini\nversion: 0.1.0\nsource: test\noverride:\n  constants.coop_bonus: { 6: 50 }\n';
		const fetchWithChild: typeof fetch = async (input) =>
			String(input).endsWith('fgc-2026-mini') ? new Response(child) : fakeFetch(input);
		const { rules } = await loadRules('fgc-2026-mini', fetchWithChild, wasm());
		expect(rules.game).toBe('fgc-2026-mini');
		expect(rules.sides.teams).toBe(3);
	});
});
