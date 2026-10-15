import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it, beforeAll } from 'vitest';
import { parse } from 'yaml';
import { initSync, RulesHandle } from '../../../crates/riso-rules/pkg/riso_rules.js';

const gamesRoot = join(__dirname, '../../../pkgs/games');

type Want = {
	score?: number;
	values?: Record<string, number | string>;
	fouls_received?: number | string;
	teams?: Record<string, number>;
};
type Case = {
	name: string;
	observation: unknown;
	expect?: { field?: Record<string, number | string>; sides: Record<string, Want> };
	expect_error?: string;
};
type Item = { name: string; value: string };

// So sánh như số thập phân: "1.10" == 1.1, 1.0 == "1". Thiếu mục thì thất bại.
const num = (v: unknown) => Number(v);
function item(items: Item[], name: string, where: string): Item {
	const found = items.find((i) => i.name === name);
	expect(found, `thiếu ${where}.${name}`).toBeDefined();
	return found!;
}

beforeAll(() => {
	initSync({
		module: readFileSync(join(__dirname, '../../../crates/riso-rules/pkg/riso_rules_bg.wasm'))
	});
});

const packs = readdirSync(gamesRoot).filter((id) => existsSync(join(gamesRoot, id, 'rules.yaml')));

it('tìm thấy ít nhất 2 đề', () => expect(packs.length).toBeGreaterThanOrEqual(2));

let total = 0;
describe.each(packs)('WASM khớp bộ test của đề %s', (id) => {
	const dir = join(gamesRoot, id);
	const handle = () => new RulesHandle(readFileSync(join(dir, 'rules.yaml'), 'utf8'), '{}');
	const files = readdirSync(join(dir, 'tests/scoring')).filter((f) => f.endsWith('.yaml'));

	total += files.length;
	it.each(files)('%s', (file) => {
		const testCase = parse(readFileSync(join(dir, 'tests/scoring', file), 'utf8')) as Case;
		const result = JSON.parse(handle().score(JSON.stringify(testCase.observation)));
		if (testCase.expect_error) {
			expect(result.ok).toBe(false);
			expect(result.errors.map((e: { code: string }) => e.code)).toContain(testCase.expect_error);
			return;
		}
		expect(result.ok, JSON.stringify(result.errors)).toBe(true);
		const sheet = result.sheet;
		for (const [name, want] of Object.entries(testCase.expect!.field ?? {})) {
			expect(num(item(sheet.field, name, 'field').value), `field.${name}`).toBe(num(want));
		}
		for (const [side, want] of Object.entries(testCase.expect!.sides)) {
			const got = sheet.sides[side];
			expect(got, `thiếu side ${side}`).toBeDefined();
			if (want.score !== undefined) expect(got.score).toBe(want.score);
			for (const [name, v] of Object.entries(want.values ?? {})) {
				expect(num(item(got.values, name, `${side}.values`).value), `${side}.${name}`).toBe(num(v));
			}
			if (want.fouls_received !== undefined) {
				expect(num(got.fouls_received), `${side}.fouls_received`).toBe(num(want.fouls_received));
			}
			for (const [id, v] of Object.entries(want.teams ?? {})) {
				const t = got.teams.find((x: { id: string }) => x.id === id);
				expect(t, `thiếu đội ${side}.${id}`).toBeDefined();
				expect(t.team_score, `${side}.${id}`).toBe(v);
			}
		}
	});
});

it('luật lỗi ném Error có danh sách RuleError', () => {
	expect(() => new RulesHandle('schema: riso.rules/v1\ngame: x', '{}')).toThrow(/RULE_YAML/);
});

it('blankObservation tính được điểm 0', () => {
	const h = new RulesHandle(readFileSync(join(gamesRoot, 'fgc-2026/rules.yaml'), 'utf8'), '{}');
	const result = JSON.parse(h.score(h.blankObservation()));
	expect(result.ok).toBe(true);
	expect(result.sheet.sides.red.score).toBe(0);
});

const fgc26 = () => readFileSync(join(gamesRoot, 'fgc-2026/rules.yaml'), 'utf8');
const caseObs = (pack: string, file: string) =>
	(parse(readFileSync(join(gamesRoot, pack, 'tests/scoring', file), 'utf8')) as Case)
		.observation as {
		game: string;
	};

it('đủ 17 ca của các đề', () => expect(total).toBe(17));

it('describe() trả RulesFile đã kế thừa', () => {
	const d = JSON.parse(new RulesHandle(fgc26(), '{}').describe());
	expect(d.game).toBe('fgc-2026');
	expect(d.sides.teams).toBe(3);
});

it('rank() xếp hạng các đội từ các biên bản', () => {
	const h = new RulesHandle(fgc26(), '{}');
	const results = ['07-cards.yaml', '01-basic.yaml'].map((f, i) => {
		const obs = caseObs('fgc-2026', f) as unknown as { sides: Record<string, { teams?: unknown }> };
		for (const side of Object.values(obs.sides)) {
			side.teams ??= [{ id: `T${i}a` }, { id: `T${i}b` }, { id: `T${i}c` }];
		}
		const scored = JSON.parse(h.score(JSON.stringify(obs)));
		expect(scored.ok, JSON.stringify(scored.errors)).toBe(true);
		return { match_id: `m${i}`, sheet: scored.sheet };
	});
	const out = JSON.parse(h.rank(JSON.stringify(results)));
	expect(out.ok, JSON.stringify(out.errors)).toBe(true);
	const teams = out.standings.rows.map((r: { team: string }) => r.team);
	for (const t of ['VIE', 'LAO', 'BRU', 'SGP', 'THA', 'PHI', 'T1a']) expect(teams).toContain(t);
	expect(out.standings.rows[0].rank).toBe(1);
});

it('đề con kế thừa đề cha qua parentsJson', () => {
	const child = [
		'schema: riso.rules/v1',
		'extends: fgc-2026',
		'game: fgc-2026-child',
		'name: Child',
		'version: 0.1.0',
		'source: "test"',
		'override:',
		'  constants.coop_bonus: { 4: 10, 5: 25, 6: 50 }'
	].join('\n');
	const h = new RulesHandle(child, JSON.stringify({ 'fgc-2026': fgc26() }));
	const obs = caseObs('fgc-2026', '05-partner-coop6.yaml');
	obs.game = 'fgc-2026-child';
	const result = JSON.parse(h.score(JSON.stringify(obs)));
	expect(result.ok, JSON.stringify(result.errors)).toBe(true);
	expect(num(item(result.sheet.field, 'coop', 'field').value)).toBe(50);
});

it('observation JSON hỏng trả OBS_JSON', () => {
	const result = JSON.parse(new RulesHandle(fgc26(), '{}').score('{không phải json'));
	expect(result.ok).toBe(false);
	expect(result.errors[0].code).toBe('OBS_JSON');
});
