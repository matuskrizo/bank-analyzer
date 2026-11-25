<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "./lib/state.svelte";
  import DropZone from "./lib/components/DropZone.svelte";
  import Filters from "./lib/components/Filters.svelte";
  import StatTiles from "./lib/components/StatTiles.svelte";
  import CategoryChart from "./lib/components/CategoryChart.svelte";
  import MonthlyChart from "./lib/components/MonthlyChart.svelte";
  import TransactionTable from "./lib/components/TransactionTable.svelte";
  import RulesPanel from "./lib/components/RulesPanel.svelte";
  import ImportsPanel from "./lib/components/ImportsPanel.svelte";

  type Tab = "overview" | "rules" | "imports";
  let tab = $state<Tab>("overview");

  const tabs: { id: Tab; label: string }[] = [
    { id: "overview", label: "Prehľad" },
    { id: "rules", label: "Pravidlá" },
    { id: "imports", label: "Importy" },
  ];

  // Pri štarte načítame dáta z Rustu.
  onMount(() => {
    app.refresh();
  });
</script>

<main>
  <header>
    <h1>Analyzátor výpisov</h1>
    <nav>
      {#each tabs as t (t.id)}
        <button class:active={tab === t.id} onclick={() => (tab = t.id)} aria-current={tab === t.id ? "page" : undefined}>
          {t.label}
          {#if t.id === "imports" && app.imports.length > 0}<span class="badge">{app.imports.length}</span>{/if}
        </button>
      {/each}
    </nav>
  </header>

  <DropZone />

  {#if app.loadError}
    <p class="error">Nepodarilo sa načítať dáta: {app.loadError}</p>
  {/if}

  {#if tab === "overview"}
    <Filters />
    <!-- počas načítania necháme starý obsah, len ho stlmíme (žiadne preblikávanie) -->
    <div class="overview" class:loading={app.loading}>
      <StatTiles />
      <div class="charts">
        <MonthlyChart />
        <CategoryChart />
      </div>
      <TransactionTable />
    </div>
  {:else if tab === "rules"}
    <RulesPanel />
  {:else}
    <ImportsPanel />
  {/if}
</main>

<style>
  main {
    max-width: 1280px;
    margin: 0 auto;
    padding: 20px 24px 40px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h1 {
    font-size: 20px;
  }

  nav {
    display: flex;
    gap: 4px;
    padding: 3px;
    border-radius: 9px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  nav button {
    border: none;
    background: none;
    min-height: 28px;
    padding: 4px 12px;
    border-radius: 6px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  nav button.active {
    background: var(--surface-raised);
    color: var(--text-primary);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
  }

  .badge {
    font-size: 11px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--hover);
    color: var(--text-secondary);
  }

  .overview {
    display: flex;
    flex-direction: column;
    gap: 14px;
    transition: opacity 0.15s;
  }

  .overview.loading {
    opacity: 0.5;
  }

  .charts {
    display: grid;
    grid-template-columns: 3fr 2fr;
    gap: 14px;
  }

  @media (max-width: 1000px) {
    .charts {
      grid-template-columns: 1fr;
    }
  }
</style>
