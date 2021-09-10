<script>
    import CheckboxBlankCircleOutline from 'svelte-material-icons/CheckboxBlankCircleOutline.svelte';
	import CheckCircle from 'svelte-material-icons/CheckCircle.svelte';

    export let item = {};
    export let onclick = (clickedId) => {}

    $: selected = false;

    export function setSelectionState(state) {
        console.log("Selection state was set to " + state + " on item " + item.id);
        selected = state;
    }

    function imageClicked(event) {
        const idStr = event.target.attributes["id"].value;
        console.log("Clicked on Element with given ID: " + idStr);
        const id = parseInt(idStr.substr(idStr.lastIndexOf('_')+1))
        selected = !selected;
        onclick(id);
    }

</script>

<div class="item">
    <div class="container">
        <div class="box">
            <div class="selectedMarker">
                {#if selected}
                    <CheckCircle color="blue" size="24" />
                {:else}
                    <CheckboxBlankCircleOutline color="blue" size="24" />
                {/if}
            </div>
        </div>
        <div class="box">
            <div id="imgCont_{item.id}" class="imageContainer" on:click={ (event) => imageClicked(event)}>
                <img id="img_{item.id}" src="/api/v1/items/load/{item.id}" alt="{item.name}" />
            </div>
        </div>
    </div>
    <div class="caption">
        <span>{item.name}</span><br>
        <span>{item.mime}</span><br>
        <span>{item.date.toDateString()}</span>
    </div>
</div>

<style>
    .item {
		width: 256px;
		margin-top: 10px;
		box-shadow: 0 2px 1px -1px rgba(0, 0, 0, 0.2), 0 1px 1px 0 rgba(0, 0, 0, 0.14), 0 1px 3px 0 rgba(0, 0, 0, 0.12);
		border-radius: 4px;
	}

	.container {
		width: 256px;
		height: 256px;
		position: relative;
	}

	.imageContainer {
		width: 256px;
		height: 256px;
		text-align: center;
		display: flex;
		justify-content: center;
		align-items: center;
		background-color: lightgrey;
		border-radius: 4px 4px 0 0;
	}

	.imageContainer:hover {
		background-color: darkgray;
	}

	.box {
		width: 100%;
		height: 100%;
		position: absolute;
		top: 0;
		left: 0;
	}

	.selectedMarker {
		z-index: 10;
		position: absolute;
		top: 0;
		right: 0;
		padding-right: 5px;
		padding-top: 5px;
		width: 32px;
		height: 32px;
		background-color: white;
		text-align: end;
		border-top-right-radius: 4px;
		border-bottom-left-radius: 24px;
	}
	 

	img {
		max-width: 256px;
		max-height: 256px;
		margin: 0;
	}

	.caption {
		padding: 5px;
	}
</style>