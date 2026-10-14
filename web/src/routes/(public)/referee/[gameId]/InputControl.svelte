<script lang="ts">
	import type { InputDef } from '$lib/riso';

	let { def, value = $bindable(), error }: { def: InputDef; value: number | string | boolean; error?: string } = $props();
	const step = (delta: number) => {
		const next = Number(value) + delta;
		if (next >= 0 && (def.max == null || next <= def.max)) value = next;
	};
</script>

<label class="flex flex-col gap-1">
	<span class="text-sm font-medium">{def.label}</span>
	{#if def.type === 'count'}
		<div class="flex items-center gap-2">
			<button type="button" class="h-12 w-12 rounded-lg border text-2xl" onclick={() => step(-1)} aria-label={`Giảm ${def.label}`}>−</button>
			<input class="h-12 w-20 rounded-lg border text-center text-xl" type="number" min="0" max={def.max ?? undefined} bind:value />
			<button type="button" class="h-12 w-12 rounded-lg border text-2xl" onclick={() => step(1)} aria-label={`Tăng ${def.label}`}>+</button>
		</div>
	{:else if def.type === 'choice'}
		<div class="flex flex-wrap gap-1" role="radiogroup" aria-label={def.label}>
			{#each def.values as option (option)}
				<button
					type="button"
					role="radio"
					aria-checked={value === option}
					class="min-h-11 rounded-lg border px-3"
					class:bg-amber-400={value === option}
					onclick={() => (value = option)}>{option}</button>
			{/each}
		</div>
	{:else}
		<input class="h-6 w-6" type="checkbox" checked={Boolean(value)} onchange={(e) => (value = e.currentTarget.checked)} />
	{/if}
	{#if error}<span class="text-sm text-red-600">{error}</span>{/if}
</label>
