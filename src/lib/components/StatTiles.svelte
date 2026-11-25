<script lang="ts">
  import { app } from "../state.svelte";
  import { money } from "../format";
</script>

<div class="tiles">
  <div class="card tile">
    <span class="label">Príjmy</span>
    <span class="value">{money(app.totals.income)}</span>
  </div>
  <div class="card tile">
    <span class="label">Výdavky</span>
    <span class="value">{money(-app.totals.expense)}</span>
  </div>
  <div class="card tile">
    <span class="label">Bilancia</span>
    <span class="value" class:positive={app.totals.balance > 0}>
      {app.totals.balance > 0 ? "+" : ""}{money(app.totals.balance)}
    </span>
  </div>
  <div class="card tile">
    <span class="label">Transakcií</span>
    <span class="value">{app.totals.count}</span>
  </div>
</div>

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
  }

  .tile {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .value {
    font-size: 24px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
</style>
