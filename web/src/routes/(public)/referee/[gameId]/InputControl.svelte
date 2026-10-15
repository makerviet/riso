<script lang="ts">
	import { stepCount, type InputDef } from '$lib/riso';

	let { def, value = $bindable(), error }: { def: InputDef; value: number | string | boolean; error?: string } = $props();
	const uid = $props.id();
	const labelId = `${uid}-label`;
	const inputId = `${uid}-input`;
	const step = (delta: number) => {
		const next = stepCount(value, delta, def.max);
		if (next !== null) value = next;
	};
</script>

<div class="flex flex-col gap-1">
	{#if def.type === 'count'}
		<div role="group" aria-labelledby={labelId} class="flex flex-col gap-1">
			<span id={labelId} class="text-sm font-medium">{def.label}</span>
			<div class="flex items-center gap-2">
				<button type="button" class="h-12 w-12 rounded-lg border text-2xl" onclick={() => step(-1)} aria-label={`Giảm ${def.label}`}>−</button>
				<input id={inputId} aria-labelledby={labelId} class="h-12 w-20 rounded-lg border border-slate-400 bg-white text-center text-xl text-slate-900" type="number" min="0" max={def.max ?? undefined} bind:value />
				<button type="button" class="h-12 w-12 rounded-lg border text-2xl" onclick={() => step(1)} aria-label={`Tăng ${def.label}`}>+</button>
			</div>
		</div>
	{:else if def.type === 'choice'}
		<fieldset class="flex flex-col gap-1">
			<legend class="text-sm font-medium">{def.label}</legend>
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
		</fieldset>
	{:else}
		<label class="flex items-center gap-2">
			<input class="h-6 w-6" type="checkbox" checked={Boolean(value)} onchange={(e) => (value = e.currentTarget.checked)} />
			<span class="text-sm font-medium">{def.label}</span>
		</label>
	{/if}
	{#if error}<span class="rounded bg-red-950/60 px-2 py-0.5 text-sm text-red-200">{error}</span>{/if}
</div>
