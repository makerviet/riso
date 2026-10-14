<script lang="ts">
	import { onMount } from 'svelte';

	type Game = { id: string; name: string; year: number; series: string };
	let games = $state<Game[]>([]);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			const response = await fetch('/api/rules');
			const body = (await response.json()) as { data: { games: Game[] } };
			games = body.data.games;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	});
</script>

<svelte:head><title>RISO — Chấm điểm trọng tài</title></svelte:head>

<main class="mx-auto max-w-2xl p-4">
	<h1 class="mb-4 text-2xl font-bold">Chấm điểm trận thật</h1>
	{#if error}<p class="text-red-600">{error}</p>{/if}
	<ul class="grid gap-2">
		{#each games as game (game.id)}
			<li>
				<a class="block rounded-lg border p-3 hover:bg-black/5" href={`/referee/${game.id}`}>
					<span class="font-semibold">{game.series} {game.year}</span> — {game.name}
				</a>
			</li>
		{/each}
	</ul>
</main>
