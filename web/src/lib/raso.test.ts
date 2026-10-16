import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { draftKey, fillFoulCounts, loadRules, stepCount } from './raso';

const games = join(__dirname, '../../../pkgs/games');
const wasm = () => readFileSync(join(__dirname, '../../../crates/raso-rules/pkg/raso_rules_bg.wasm'));
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
		expect(draftKey('fgc-2026')).toBe('raso-referee-draft:fgc-2026');
	});

	it('tải đề cha theo extends', async () => {
		const child =
			'schema: raso.rules/v1\nextends: fgc-2026\ngame: fgc-2026-mini\nname: Mini\nversion: 0.1.0\nsource: test\noverride:\n  constants.coop_bonus: { 6: 50 }\n';
		const fetchWithChild: typeof fetch = async (input) =>
			String(input).endsWith('fgc-2026-mini') ? new Response(child) : fakeFetch(input);
		const { rules } = await loadRules('fgc-2026-mini', fetchWithChild, wasm());
		expect(rules.game).toBe('fgc-2026-mini');
		expect(rules.sides.teams).toBe(3);
	});

	it('ô số bị xoá (null) báo OBS_RANGE đúng đường dẫn', async () => {
		const { handle } = await loadRules('fgc-2026', fakeFetch, wasm());
		const obs = JSON.parse(handle.blankObservation());
		for (const side of Object.values(obs.sides) as { teams?: unknown }[]) side.teams = [];
		obs.sides.red.su_wildfire = null;
		const r = JSON.parse(handle.score(JSON.stringify(obs)));
		expect(r.ok).toBe(false);
		expect(r.errors).toContainEqual(
			expect.objectContaining({ path: 'sides.red.su_wildfire', code: 'OBS_RANGE' })
		);
	});

	it('biên bản trống có sẵn 0 cho mọi lỗi, nên +/− lỗi chạy được', async () => {
		const { handle, rules } = await loadRules('fgc-2026', fakeFetch, wasm());
		const obs = JSON.parse(handle.blankObservation());
		const ids = Object.keys(rules.fouls);
		expect(ids.length).toBeGreaterThan(0);
		for (const side of Object.values(obs.sides) as { fouls: Record<string, number> }[]) {
			expect(side.fouls).toEqual(Object.fromEntries(ids.map((id) => [id, 0])));
			expect(stepCount(side.fouls[ids[0]], 1, 99)).toBe(1);
		}
	});

	it('ô lỗi bị xoá (null) báo OBS_RANGE đúng đường dẫn, không hỏng cả biên bản', async () => {
		const { handle, rules } = await loadRules('fgc-2026', fakeFetch, wasm());
		const obs = JSON.parse(handle.blankObservation());
		for (const side of Object.values(obs.sides) as { teams?: unknown }[]) side.teams = [];
		const id = Object.keys(rules.fouls)[0];
		obs.sides.red.fouls[id] = null;
		const r = JSON.parse(handle.score(JSON.stringify(obs)));
		expect(r.ok).toBe(false);
		expect(r.errors).toEqual([expect.objectContaining({ path: `sides.red.fouls.${id}`, code: 'OBS_RANGE' })]);
		expect(r.errors[0].message).not.toMatch(/Text\(/);
	});
});

describe('stepCount / fillFoulCounts', () => {
	it('ô rỗng hoặc thiếu coi như 0, chặn ngoài khoảng', () => {
		expect(stepCount(undefined, 1)).toBe(1);
		expect(stepCount(null, 1)).toBe(1);
		expect(stepCount('', 1)).toBe(1);
		expect(stepCount(3, -1)).toBe(2);
		expect(stepCount(0, -1)).toBeNull();
		expect(stepCount(99, 1, 99)).toBeNull();
	});

	it('bản nháp cũ thiếu fouls được bổ sung 0, giữ số đã nhập', () => {
		const sides: Record<string, { fouls?: Record<string, unknown> }> = { red: {}, blue: { fouls: { minor: 2 } } };
		fillFoulCounts(sides, ['minor', 'major']);
		expect(sides.red.fouls).toEqual({ minor: 0, major: 0 });
		expect(sides.blue.fouls).toEqual({ minor: 2, major: 0 });
	});
});
