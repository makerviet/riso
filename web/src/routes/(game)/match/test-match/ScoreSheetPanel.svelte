<script lang="ts">
	import type { ScoreSheetSection } from './match-protocol';

	let { data }: { data: ScoreSheetSection | null } = $props();
	const sides = $derived(data?.sheet ? Object.entries(data.sheet.sides) : []);
	const fieldItems = $derived(data?.sheet?.field.filter((item) => item.label) ?? []);
</script>

{#if data}
	<section
		class="pointer-events-auto max-w-sm rounded-lg bg-black/70 p-3 text-xs text-white"
		aria-label="Chi tiết điểm"
	>
		<h2 class="mb-2 text-sm font-semibold">Chi tiết điểm</h2>
		{#if data.scoringError}
			<p class="mb-2 rounded bg-red-700/80 p-2">Lỗi tính điểm: {data.scoringError}</p>
		{/if}
		{#each sides as [id, side] (id)}
			<div class="mb-2">
				<div
					class="flex justify-between font-semibold"
					class:text-red-300={id === 'red'}
					class:text-sky-300={id === 'blue'}
				>
					<span>{id.toUpperCase()}</span><span>{side.score}</span>
				</div>
				{#each side.values.filter((item) => item.label) as item (item.name)}
					<div class="flex justify-between gap-4 opacity-90">
						<span>{item.label}</span><span>{item.value}</span>
					</div>
				{/each}
			</div>
		{/each}
		{#each fieldItems as item (item.name)}
			<div class="flex justify-between gap-4 opacity-90">
				<span>{item.label}</span><span>{item.value}</span>
			</div>
		{/each}
		{#if data.sheet?.not_simulated.length}
			<p class="mt-2 opacity-70">Chưa mô phỏng: {data.sheet.not_simulated.join(', ')}</p>
		{/if}
	</section>
{/if}
