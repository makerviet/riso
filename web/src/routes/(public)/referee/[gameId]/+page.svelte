<script lang="ts">
	import { onMount } from 'svelte';
	import InputControl from './InputControl.svelte';
	import { draftKey, fillFoulCounts, loadRules, type RulesDescription } from '$lib/riso';
	import type { RulesHandle } from '$riso/riso_rules.js';

	let { data } = $props();
	type Obs = {
		game: string;
		rules_version?: string;
		field: Record<string, number | string | boolean>;
		sides: Record<string, Record<string, unknown> & {
			robots: Record<string, number | string | boolean>[];
			fouls: Record<string, number>;
			teams: { id: string; card?: string | null }[];
		}>;
	};

	let handle: RulesHandle | null = null;
	let rules = $state<RulesDescription | null>(null);
	let obs = $state<Obs | null>(null);
	let loadError = $state<string | null>(null);

	onMount(async () => {
		try {
			const loaded = await loadRules(data.gameId);
			handle = loaded.handle;
			rules = loaded.rules;
			let draft: Obs | null = null;
			try {
				const saved = localStorage.getItem(draftKey(data.gameId));
				draft = saved ? (JSON.parse(saved) as Obs) : null;
			} catch {
				draft = null;
			}
			const blank = JSON.parse(handle.blankObservation()) as Obs;
			for (const side of Object.values(blank.sides)) {
				side.teams = Array.from({ length: loaded.rules.sides.teams }, () => ({ id: '', card: null }));
			}
			if (draft?.game === blank.game && draft.rules_version === loaded.rules.version) {
				const { rules_version: _v, ...rest } = draft;
				fillFoulCounts(rest.sides, Object.keys(loaded.rules.fouls));
				obs = rest;
			} else {
				fillFoulCounts(blank.sides, Object.keys(loaded.rules.fouls));
				obs = blank;
			}
		} catch (e) {
			loadError = e instanceof Error ? e.message : String(e);
		}
	});

	const forScoring = (o: Obs) => ({
		...o,
		sides: Object.fromEntries(
			Object.entries(o.sides).map(([id, s]) => [id, { ...s, teams: s.teams.filter((t) => t.id.trim() !== '') }])
		)
	});

	type ScoreResult = {
		ok: boolean;
		errors: { path: string; code: string; message: string }[];
		sheet: {
			sides: Record<
				string,
				{
					score: number | string;
					raw_score: number | string;
					fouls_received: number | string;
					values: { name: string; label: string | null; value: number | string }[];
					teams: { id: string; card?: string | null; team_score: number | string }[];
				}
			>;
			field: { name: string; label: string | null; value: number | string }[];
		};
	};

	const result = $derived.by((): ScoreResult | null => {
		if (!handle || !obs) return null;
		return JSON.parse(handle.score(JSON.stringify(forScoring(obs))));
	});

	$effect(() => {
		if (!obs) return;
		try {
			localStorage.setItem(draftKey(data.gameId), JSON.stringify({ ...obs, rules_version: rules?.version }));
		} catch {
			/* trình duyệt chặn lưu trữ: bỏ qua, trang vẫn chạy */
		}
	});

	const errorFor = (path: string) =>
		result && !result.ok ? result.errors.find((e: { path: string }) => e.path === path)?.message : undefined;

	function reset() {
		if (!handle || !rules) return;
		if (!window.confirm('Xoá toàn bộ biên bản trận này?')) return;
		const blank = JSON.parse(handle.blankObservation()) as Obs;
		for (const side of Object.values(blank.sides)) {
			side.teams = Array.from({ length: rules.sides.teams }, () => ({ id: '', card: null }));
		}
		fillFoulCounts(blank.sides, Object.keys(rules.fouls));
		obs = blank;
	}

	function download() {
		if (!obs || !result?.ok) return;
		const blob = new Blob([JSON.stringify({ observation: forScoring(obs), sheet: result.sheet }, null, 2)], { type: 'application/json' });
		const a = document.createElement('a');
		a.href = URL.createObjectURL(blob);
		a.download = `bien-ban-${data.gameId}-${new Date().toISOString().replace(/[:.]/g, '-')}.json`;
		a.click();
		URL.revokeObjectURL(a.href);
	}
</script>

<svelte:head><title>RISO — Trọng tài {data.gameId}</title></svelte:head>

<main class="mx-auto max-w-6xl p-4 print:p-0">
	{#if loadError}
		<p class="rounded bg-red-950/60 px-3 py-2 text-red-200">{loadError}</p>
	{:else if !rules || !obs}
		<p>Đang tải luật…</p>
	{:else}
		<header class="mb-4 flex flex-wrap items-center justify-between gap-2">
			<h1 class="text-xl font-bold">{rules.name} <span class="text-sm font-normal opacity-70">v{rules.version}</span></h1>
			<div class="flex gap-2 print:hidden">
				<button class="rounded-lg border px-3 py-2" onclick={reset}>Xoá biên bản</button>
				<button class="rounded-lg border px-3 py-2" onclick={() => window.print()}>In</button>
				<button class="rounded-lg border px-3 py-2" disabled={!result?.ok} onclick={download}>Tải JSON</button>
			</div>
		</header>

		<div class="grid gap-4 lg:grid-cols-[2fr_1fr]">
			<div class="grid gap-4">
				{#if Object.keys(rules.inputs.field).length}
					<section class="grid gap-3 rounded-lg border p-3 sm:grid-cols-2">
						<h2 class="font-semibold sm:col-span-2">Chung toàn sân</h2>
						{#each Object.entries(rules.inputs.field) as [name, def] (name)}
							<InputControl {def} bind:value={obs.field[name]} error={errorFor(`field.${name}`)} />
						{/each}
					</section>
				{/if}
				<div class="grid gap-4 md:grid-cols-2">
					{#each rules.sides.ids as side (side)}
						<section class="grid gap-3 rounded-lg border p-3" class:border-red-500={side === 'red'} class:border-sky-500={side === 'blue'}>
							<h2 class="font-semibold uppercase">{side}</h2>
							{#each Object.entries(rules.inputs.side) as [name, def] (name)}
								<InputControl {def} bind:value={obs.sides[side][name] as number} error={errorFor(`sides.${side}.${name}`)} />
							{/each}
							{#each obs.sides[side].robots as robot, i (i)}
								<fieldset class="grid gap-2 rounded border p-2">
									<legend class="px-1 text-sm">Robot {i + 1}</legend>
									<input class="rounded border border-slate-400 bg-white px-2 py-1 text-slate-900 placeholder:text-slate-500" placeholder="Mã đội" bind:value={obs.sides[side].teams[i].id} />
									{#each Object.entries(rules.inputs.robot) as [name, def] (name)}
										<InputControl {def} bind:value={robot[name]} error={errorFor(`sides.${side}.robots.${i}.${name}`)} />
									{/each}
									{#if Object.keys(rules.cards).length}
										<select class="rounded border border-slate-400 bg-white px-2 py-1 text-slate-900" bind:value={obs.sides[side].teams[i].card}>
											<option value={null}>Không thẻ</option>
											{#each Object.keys(rules.cards) as card (card)}<option value={card}>Thẻ {card}</option>{/each}
										</select>
									{/if}
								</fieldset>
							{/each}
							{#each Object.entries(rules.fouls) as [id, foul] (id)}
								<InputControl
									def={{ type: 'count', values: [], label: foul.label, max: 99 }}
									bind:value={obs.sides[side].fouls[id]}
									error={errorFor(`sides.${side}.fouls.${id}`)} />
							{/each}
						</section>
					{/each}
				</div>
			</div>

			<aside class="h-fit rounded-lg border p-3 lg:sticky lg:top-4" aria-live="polite">
				<h2 class="mb-2 font-semibold">Bảng điểm</h2>
				{#if result?.ok}
					{#each Object.entries(result.sheet.sides) as [side, s] (side)}
						<div class="mb-3">
							<div class="flex justify-between text-lg font-bold"><span class="uppercase">{side}</span><span>{s.score}</span></div>
							{#each s.values.filter((v: { label: string | null }) => v.label) as v (v.name)}
								<div class="flex justify-between text-sm"><span>{v.label}</span><span>{v.value}</span></div>
							{/each}
							<div class="flex justify-between text-sm"><span>Phạt nhận từ đối thủ</span><span>{s.fouls_received}</span></div>
							<div class="flex justify-between text-sm opacity-70"><span>Điểm thô</span><span>{s.raw_score}</span></div>
							{#each s.teams as t (t.id)}
								<div class="flex justify-between text-sm"><span>Đội {t.id}{t.card ? ` (thẻ ${t.card})` : ''}</span><span>{t.team_score}</span></div>
							{/each}
						</div>
					{/each}
					{#each result.sheet.field.filter((v: { label: string | null }) => v.label) as v (v.name)}
						<div class="flex justify-between text-sm"><span>{v.label}</span><span>{v.value}</span></div>
					{/each}
				{:else if result}
					<ul class="grid gap-1 rounded bg-red-950/60 p-2 text-sm text-red-200">
						{#each result.errors as e (e.path + e.code)}<li>{e.path}: {e.message}</li>{/each}
					</ul>
				{/if}
			</aside>
		</div>
	{/if}
</main>
