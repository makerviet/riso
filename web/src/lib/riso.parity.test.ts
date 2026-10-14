import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it, beforeAll } from 'vitest';
import { parse } from 'yaml';
import { initSync, RulesHandle } from '../../../crates/riso-rules/pkg/riso_rules.js';

const gamesRoot = join(__dirname, '../../../pkgs/games');

type Case = {
	name: string;
	observation: unknown;
	expect?: { sides: Record<string, { score?: number }> };
	expect_error?: string;
};

beforeAll(() => {
	initSync({
		module: readFileSync(join(__dirname, '../../../crates/riso-rules/pkg/riso_rules_bg.wasm'))
	});
});

const packs = readdirSync(gamesRoot).filter((id) => existsSync(join(gamesRoot, id, 'rules.yaml')));

describe.each(packs)('WASM khớp bộ test của đề %s', (id) => {
	const dir = join(gamesRoot, id);
	const handle = () => new RulesHandle(readFileSync(join(dir, 'rules.yaml'), 'utf8'), '{}');
	const files = readdirSync(join(dir, 'tests/scoring')).filter((f) => f.endsWith('.yaml'));

	it.each(files)('%s', (file) => {
		const testCase = parse(readFileSync(join(dir, 'tests/scoring', file), 'utf8')) as Case;
		const result = JSON.parse(handle().score(JSON.stringify(testCase.observation)));
		if (testCase.expect_error) {
			expect(result.ok).toBe(false);
			expect(result.errors.map((e: { code: string }) => e.code)).toContain(testCase.expect_error);
			return;
		}
		expect(result.ok, JSON.stringify(result.errors)).toBe(true);
		for (const [side, want] of Object.entries(testCase.expect!.sides)) {
			if (want.score !== undefined) expect(result.sheet.sides[side].score).toBe(want.score);
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
