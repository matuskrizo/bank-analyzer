<script lang="ts">
  // Zoznam importov. Zmazanie importu zmaže aj jeho transakcie
  // (v databáze ON DELETE CASCADE). Mazanie má dva kroky, aby sa
  // nedalo urobiť omylom.
  import { app } from "../state.svelte";
  import { errorMessage } from "../api";

  let confirming = $state<number | null>(null);
  let message = $state<string | null>(null);

  async function remove(id: number) {
    try {
      await app.deleteImport(id);
      confirming = null;
      message = null;
    } catch (err) {
      message = errorMessage(err);
    }
  }
</script>

<section class="card">
  <h2>Importy</h2>
  {#if message}<p class="error">{message}</p>{/if}
  {#if app.imports.length === 0}
    <p class="muted">Zatiaľ nič. Pretiahni výpis do okna.</p>
  {:else}
    <table>
      <thead>
        <tr><th>Súbor</th><th>Formát</th><th>Importované</th><th class="num">Transakcií</th><th></th></tr>
      </thead>
      <tbody>
        {#each app.imports as item (item.id)}
          <tr>
            <td>{item.fileName}</td>
            <td class="secondary">{item.format}</td>
            <td class="secondary">{item.importedAt}</td>
            <td class="num">{item.rowCount}</td>
            <td class="actions">
              {#if confirming === item.id}
                <button class="danger" onclick={() => remove(item.id)}>Naozaj zmazať</button>
                <button onclick={() => (confirming = null)}>Zrušiť</button>
              {:else}
                <button onclick={() => (confirming = item.id)}>Zmazať</button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  h2 {
    font-size: 15px;
    margin-bottom: 10px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    text-align: left;
    font-weight: 500;
    font-size: 13px;
    color: var(--text-muted);
    padding: 6px 8px;
    border-bottom: 1px solid var(--axis);
  }

  td {
    padding: 7px 8px;
    border-bottom: 1px solid var(--grid);
  }

  .actions {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
  }

  .danger {
    color: var(--critical);
    border-color: var(--critical);
  }
</style>
