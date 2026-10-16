<script lang="ts">
	import { resolve } from '$app/paths';
	import { dynamicRoute } from '$lib/paths';
	import { page } from '$app/state';
	import { locales, localizeHref } from '$lib/paraglide/runtime';
	import './layout.css';

	let { children } = $props();
</script>

{@render children()}

<div style="display:none">
	{#each locales as locale (locale)}
		<a href={resolve(dynamicRoute(localizeHref(page.url.pathname, { locale })))}>{locale}</a>
	{/each}
</div>
