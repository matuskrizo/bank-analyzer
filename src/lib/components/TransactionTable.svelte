<script lang="ts">
  // Tabuľka transakcií s ručnou zmenou kategórie.
  // Po ručnej zmene ponúkneme vytvorenie pravidla, aby sa podobné
  // platby zaradili automaticky aj v budúcnosti.
  import { app } from "../state.svelte";
  import { date, money } from "../format";
  import { errorMessage, type Transaction } from "../api";

  const PAGE = 200;
  let limit = $state(PAGE);
  let visible = $derived(app.filtered.slice(0, limit));

  // Po zmene filtrov začni znova od prvej strany.
  $effect(() => {
    void app.filtered; // čítaním hodnoty si $effect zaregistruje závislosť
    limit = PAGE;
  });

  let suggestion = $state<{ pattern: string; categoryId: number; categoryName: string } | null>(null);
  let message = $state<string | null>(null);

  async function changeCategory(tx: Transaction, value: string) {
    const categoryId = value === "" ? null : Number(value);
    message = null;
    try {
      await app.setCategory(tx.id, categoryId);
      const category = app.categories.find((c) => c.id === categoryId);
      const pattern = tx.counterparty ?? tx.description ?? "";
      suggestion = category && pattern ? { pattern, categoryId: category.id, categoryName: category.name } : null;
    } catch (err) {
      message = errorMessage(err);
    }
  }

  async function createRule() {
    if (!suggestion) return;
    try {
      const changed = await app.addRule(suggestion.pattern, suggestion.categoryId);
      message = `Pravidlo „${suggestion.pattern}“ uložené · preradených transakcií: ${changed}`;
      suggestion = null;
    } catch (err) {
      message = errorMessage(err);
    }
  }
</script>

<section class="card">
  <h2>Transakcie</h2>

  {#if suggestion}
    <div class="suggest">
      Zaraďovať aj ďalšie platby obsahujúce
      <input bind:value={suggestion.pattern} aria-label="Vzor pravidla" />
      do kategórie <strong>{suggestion.categoryName}</strong>?
      <button class="primary" onclick={createRule}>Vytvoriť pravidlo</button>
      <button onclick={() => (suggestion = null)}>Nie</button>
    </div>
  {/if}
  {#if message}<p class="secondary msg">{message}</p>{/if}

  {#if app.filtered.length === 0}
    <p class="muted empty">
      {app.transactions.length === 0 ? "Zatiaľ žiadne transakcie – importuj výpis." : "Filtrom nevyhovuje žiadna transakcia."}
    </p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Dátum</th>
          <th>Protistrana a popis</th>
          <th>Kategória</th>
          <th class="num">Suma</th>
        </tr>
      </thead>
      <tbody>
        <!-- (tx.id) je kľúč: Svelte podľa neho pozná, ktorý riadok je ktorý -->
        {#each visible as tx (tx.id)}
          <tr>
            <td class="nowrap secondary">{date(tx.date)}</td>
            <td>
              <div>{tx.counterparty ?? "—"}</div>
              {#if tx.description || tx.variableSymbol}
                <div class="muted small">
                  {tx.description ?? ""}{#if tx.variableSymbol}{tx.description ? " · " : ""}VS {tx.variableSymbol}{/if}
                </div>
              {/if}
            </td>
            <td>
              <select
                value={tx.categoryId === null ? "" : String(tx.categoryId)}
                onchange={(e) => changeCategory(tx, e.currentTarget.value)}
                aria-label="Kategória transakcie"
                class:manual={tx.categoryManual}
                title={tx.categoryManual ? "Nastavené ručne" : "Zaradené automaticky"}
              >
                <option value="">Nezaradené</option>
                {#each app.categories as category (category.id)}
                  <option value={String(category.id)}>{category.name}</option>
                {/each}
              </select>
            </td>
            <td class="num" class:positive={tx.amountCents > 0}>
              {tx.amountCents > 0 ? "+" : ""}{money(tx.amountCents)}
              {#if tx.currency !== "EUR"}<span class="muted small">{tx.currency}</span>{/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if app.filtered.length > limit}
      <button class="more" onclick={() => (limit += PAGE)}>
        Zobraziť ďalšie ({app.filtered.length - limit})
      </button>
    {/if}
  {/if}
</section>

<style>
  h2 {
    font-size: 15px;
    margin-bottom: 10px;
  }

  .empty {
    margin: 24px 0;
  }

  .suggest {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    margin-bottom: 10px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--accent) 8%, var(--surface));
  }

  .msg {
    margin: 0 0 10px;
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
    vertical-align: top;
  }

  tbody tr:hover {
    background: var(--hover);
  }

  .nowrap {
    white-space: nowrap;
  }

  .small {
    font-size: 12.5px;
  }

  select {
    min-width: 170px;
    padding: 3px 6px;
    min-height: 28px;
  }

  select.manual {
    border-color: var(--accent);
  }

  .more {
    margin-top: 10px;
  }
</style>
