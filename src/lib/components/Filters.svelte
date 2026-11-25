<script lang="ts">
  // Filtre zapisujú priamo do `app.filters` ($state). Všetko, čo je
  // odvodené cez $derived (tabuľka, grafy, súčty), sa prepočíta samo.
  import { app, type CategoryFilter } from "../state.svelte";
  import { datePresets } from "../format";

  const presets = datePresets();

  // Ktorá predvoľba zodpovedá aktuálnemu rozsahu (ak nejaká).
  let activePreset = $derived(
    presets.find((p) => p.from === app.filters.from && p.to === app.filters.to)?.label ?? "",
  );

  // <select> pracuje s textom; prevedieme ho na CategoryFilter.
  function onCategoryChange(e: Event) {
    const value = (e.currentTarget as HTMLSelectElement).value;
    app.filters.category = (value === "all" || value === "none" ? value : Number(value)) as CategoryFilter;
  }
</script>

<div class="filters" role="search">
  <select
    aria-label="Obdobie"
    value={activePreset}
    onchange={(e) => {
      const p = presets.find((p) => p.label === e.currentTarget.value);
      if (p) Object.assign(app.filters, { from: p.from, to: p.to });
    }}
  >
    {#if activePreset === ""}<option value="">Vlastné obdobie</option>{/if}
    {#each presets as preset (preset.label)}
      <option value={preset.label}>{preset.label}</option>
    {/each}
  </select>

  <!-- bind:value = obojsmerné prepojenie inputu so stavom -->
  <label>od <input type="date" bind:value={app.filters.from} /></label>
  <label>do <input type="date" bind:value={app.filters.to} /></label>

  <select aria-label="Kategória" value={String(app.filters.category)} onchange={onCategoryChange}>
    <option value="all">Všetky kategórie</option>
    <option value="none">Nezaradené</option>
    {#each app.categories as category (category.id)}
      <option value={String(category.id)}>{category.name}</option>
    {/each}
  </select>

  <select aria-label="Smer" bind:value={app.filters.direction}>
    <option value="all">Príjmy aj výdavky</option>
    <option value="out">Len výdavky</option>
    <option value="in">Len príjmy</option>
  </select>

  <input type="search" placeholder="Hľadať protistranu, popis, VS…" bind:value={app.filters.search} />

  {#if app.hasActiveFilters}
    <button class="link" onclick={() => app.resetFilters()}>Zrušiť filtre</button>
  {/if}
</div>

<style>
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
  }

  input[type="search"] {
    flex: 1;
    min-width: 200px;
  }
</style>
