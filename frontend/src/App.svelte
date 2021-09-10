<svelte:head>
	<title>Filebase</title>
</svelte:head>

<div class="actionBar">
	<div class="selectionInfo">
		<h2>Selected {selectedCnt} of {items.length}</h2>
	</div>
	{#if errorVisible}
		<div class="backendError" >
			<h2>{errorMsg}</h2>
		</div>
	{:else}
		<div class="spacer"><div>.</div></div>
	{/if}
	<div class="buttonGroup">
		{#each destinations as d}
			<button id="confirm_{d.id}" class="confirmButton" on:click={confirm}>{d.name}</button>
		{/each}
		<button id="discard" on:click={discardSelection}>Discard</button>
	</div>
</div>
<div class="content">
	<div class="photos">
		{#each items as item}
			<ItemCard item={item} onclick={(id) => selectionChangedOnItem(id)} bind:this={item.ref} />
		{/each}
	</div>
</div>

<style>
	.actionBar {
		overflow: hidden;
		background-color: #333;
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		z-index: 25;
		padding-left: 10px;
	}

	.content {
		padding-top: 90px;
		padding-bottom: 20px;
	}

	.photos {
		width: 100%;
		margin: 0 auto;
		display: grid;
		grid-gap: 1rem;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
	}

	.selectionInfo {
		width: 33%;
		color: white;
		float: left;
		display: block;
	}

	.backendError {
		width: 33%;
		height: 100%;
		color: white;
		text-align: center;
		float: left;
		display: block;
		background-color: darkred;
	}

	.spacer {
		width: 33%;
		float: left;
		display: block;
	}

	.buttonGroup {
		width: 33%;
		text-align: end;
		display: block;
		float: left;
	}

	button {
		margin-top: 12px;
		margin-bottom: 12px;
		color: white;
		padding: 15px 32px;
		text-align: center;
		text-decoration: none;
		font-size: 16px;
		cursor: pointer;
		display: inline-block;
	}

	#discard {
		margin-left: 32px;
		margin-right: 24px;
		background-color: darkred;
		border: 1px solid darkmagenta;
	}

	#discard:hover {
		background-color: red;
	}

	.confirmButton {
		background-color: #4CAF50;
		border: 1px solid green;
	}

	 .confirmButton:not(:last-child) {
		border-right: none;
	}

	.confirmButton:hover {
		background-color: #3e8e41;
	}

</style>

<script lang="ts">
	import {onMount} from 'svelte';
	import ItemCard from './components/ItemCard.svelte';

	let items = [];
	let destinations = [];
	let selectedIds = [];
	$: selectedCnt = 0;
	$: itemCount = 0;
	let errorMsg = '';
	$: errorVisible = errorMsg !== '';


	async function loadItems() {
		const res = await fetch('/api/v1/items');
		let it = await res.json();
		it = itemsConvertDate(it);
		it.sort((elem1, elem2) => elem1.creation_date - elem2.creation_date);
		items = it;
		itemCount = items.length;
	}

	async function loadDestinations() {
		const res = await fetch('/api/v1/destinations');
		destinations = await res.json();
	}

	function itemsConvertDate(it) {
		for(let i = 0; i < it.length; i++) {
			it[i].date = convertDate(it[i].creation_date)
		}
		return it;
	}

	function convertDate(ms) {
		//date.setUTCSeconds(seconds);
		return new Date(ms);
	}

	function selectionChangedOnItem(id) {
		if(selectedIds.includes(id)) {
			//element was deselected
			console.log("Deselected id " + id);
			selectedIds = selectedIds.filter((elem) => elem !== id);
		}else{
			//element was selected
			console.log("Selected Id " + id);
			selectedIds.push(id);
		}
		console.log(selectedIds);
		selectedCnt = selectedIds.length;
	}

	async function confirm(event) {
		const idStr = event.target.attributes["id"].value;
		const id = parseInt(idStr.substr(idStr.lastIndexOf('_')+1));
		console.log("Confirming to Id " + id);
		await confirmSelection(id);
	}

	async function confirmSelection(destinationId) {
		const response = await fetch('/api/v1/items/confirm', {
			method: 'POST',
			cache: 'no-cache',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({
				"destination" : destinationId,
				"ids" : selectedIds
			})
		});
		if(response.ok) {
			console.log("Successfully confirmed items");
			removeHandledItems();
		}else{
			console.log(response.body);
			const errors = await response.json();
			console.error(errors);
			errorMsg = 'Received Errors. Reload.';
		}
	}

	async function discardSelection() {
		const response = await fetch('/api/v1/items/discard', {
			method: 'POST',
			cache: 'no-cache',
			headers: {
				'Content-Type' : 'application/json'
			},
			body: JSON.stringify({
				"ids" : selectedIds
			})
		});
		if(response.ok) {
			console.log("Successfully discarded!");
			removeHandledItems();
		}else{
			console.log(response.body);
			const errors = await response.json();
			console.error(errors);
			errorMsg = 'Received Errors. Reload.';
		}
	}

	function removeHandledItems() {
		console.log("Removing items");
		console.log("Current Item Count: " + items.length);
		for(const item of items) {
			item.ref.setSelectionState(false);
		}
		items = items.filter((elem) => {
			const inSelection = selectedIds.includes(elem.id);
			return !inSelection;
		});

		itemCount = items.length;
		selectedIds = [];
		selectedCnt = 0;
		console.log("New item count is " + itemCount);
	}

	onMount(async () => {
		await loadItems();
		await loadDestinations();
	});
</script>