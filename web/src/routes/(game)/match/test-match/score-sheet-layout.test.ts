import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { describe, expect, it } from 'vitest';

// Hồi quy: bảng "Chi tiết điểm" từng nằm trong một khối absolute riêng
// (top-24 left-3 z-10) ngay dưới thẻ "Live match" (top-4 left-4 z-10, đứng sau
// trong DOM) nên bị thẻ HUD che mất tiêu đề và phần đầu bảng. Hai khối phải
// cùng nằm trong một cột flex để xếp nối tiếp, không chồng lên nhau.

type Node = {
	type: string;
	name?: string;
	attributes?: { name: string; value: unknown }[];
	children?: Node[];
	fragment?: { nodes: Node[] };
	[key: string]: unknown;
};

const source = readFileSync(new URL('./+page.svelte', import.meta.url), 'utf8');
const ast = parse(source, { modern: true }) as unknown as { fragment: { nodes: Node[] } };

function classOf(node: Node): string {
	const attribute = node.attributes?.find((entry) => entry.name === 'class');
	if (!attribute || !Array.isArray(attribute.value)) return '';
	return attribute.value
		.map((part: { type: string; data?: string }) => (part.type === 'Text' ? part.data : ''))
		.join('');
}

function childNodes(node: Node): Node[] {
	const nested: Node[] = [];
	for (const value of Object.values(node)) {
		if (value && typeof value === 'object' && 'nodes' in (value as object)) {
			nested.push(...((value as { nodes: Node[] }).nodes ?? []));
		}
	}
	return nested;
}

function findPath(nodes: Node[], match: (node: Node) => boolean, path: Node[] = []): Node[] | null {
	for (const node of nodes) {
		const next = [...path, node];
		if (match(node)) return next;
		const found = findPath(childNodes(node), match, next);
		if (found) return found;
	}
	return null;
}

const isPositioned = (node: Node) =>
	node.type === 'RegularElement' && /\b(absolute|fixed)\b/.test(classOf(node));

describe('test-match score sheet layout', () => {
	it('stacks the score sheet in the same left column as the Live match HUD', () => {
		const panelPath = findPath(
			ast.fragment.nodes,
			(node) => node.type === 'Component' && node.name === 'ScoreSheetPanel'
		);
		const hudPath = findPath(
			ast.fragment.nodes,
			(node) =>
				node.type === 'RegularElement' &&
				node.name === 'p' &&
				childNodes(node).some(
					(child) => child.type === 'Text' && String(child.data).trim() === 'Live match'
				)
		);
		expect(panelPath).not.toBeNull();
		expect(hudPath).not.toBeNull();

		const panelAnchor = panelPath!.filter(isPositioned).at(-1);
		const hudAnchor = hudPath!.filter(isPositioned).at(-1);
		expect(panelAnchor).toBeDefined();
		// Cùng một khối định vị: cột flex xếp HUD rồi bảng điểm, không chồng lớp.
		expect(panelAnchor).toBe(hudAnchor);
		expect(classOf(panelAnchor!)).toMatch(/\bflex-col\b/);
	});
});
