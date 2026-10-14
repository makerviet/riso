import init, { initSync, RulesHandle } from '$riso/riso_rules.js';

export type InputDef = {
	type: 'count' | 'choice' | 'bool';
	max?: number | null;
	values: string[];
	label: string;
	label_en?: string | null;
	help?: string | null;
};

export type RulesDescription = {
	game: string;
	name: string;
	version: string;
	source: string;
	sides: { ids: string[]; teams: number };
	inputs: { field: Record<string, InputDef>; side: Record<string, InputDef>; robot: Record<string, InputDef> };
	fouls: Record<string, { label: string; label_en?: string | null }>;
	cards: Record<string, string>;
};

let ready: Promise<void> | null = null;

function ensureWasm(wasmBytes?: BufferSource): Promise<void> {
	if (wasmBytes) {
		initSync({ module: wasmBytes });
		return Promise.resolve();
	}
	ready ??= init().then(() => undefined);
	return ready;
}

async function fetchYaml(gameId: string, fetchFn: typeof fetch): Promise<string> {
	const response = await fetchFn(`/api/rules/${encodeURIComponent(gameId)}`);
	if (!response.ok) throw new Error(`Không tải được luật của đề '${gameId}' (${response.status})`);
	return response.text();
}

/** Tải luật + các đề cha (tối đa 4 tầng) rồi khởi tạo `RulesHandle`. */
export async function loadRules(
	gameId: string,
	fetchFn: typeof fetch = fetch,
	wasmBytes?: BufferSource
): Promise<{ handle: RulesHandle; rules: RulesDescription }> {
	await ensureWasm(wasmBytes);
	const yaml = await fetchYaml(gameId, fetchFn);
	const parents: Record<string, string> = {};
	let current = yaml;
	for (let depth = 0; depth < 4; depth += 1) {
		const parent = /^extends:\s*([\w-]+)\s*$/m.exec(current)?.[1];
		if (!parent || parents[parent]) break;
		current = await fetchYaml(parent, fetchFn);
		parents[parent] = current;
	}
	let handle: RulesHandle;
	try {
		handle = new RulesHandle(yaml, JSON.stringify(parents));
	} catch (error) {
		throw new Error(`Luật của đề '${gameId}' không hợp lệ: ${error instanceof Error ? error.message : String(error)}`);
	}
	return { handle, rules: JSON.parse(handle.describe()) as RulesDescription };
}

export function draftKey(gameId: string): string {
	return `riso-referee-draft:${gameId}`;
}
