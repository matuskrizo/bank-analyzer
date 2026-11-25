<script lang="ts">
  // Správa pravidiel "vzor → kategória" a vlastných kategórií.
  import { app } from "../state.svelte";
  import { errorMessage } from "../api";

  let pattern = $state("");
  let categoryId = $state<number | null>(null);
  let newCategory = $state("");
  let message = $state<string | null>(null);

  // Pravidlá zoskupené podľa kategórie – $derived sa prepočíta po každom refresh().
  let grouped = $derived.by(() => {
    const groups = new Map<string, typeof app.rules>();
    for (const rule of app.rules) {
      groups.set(rule.category, [...(groups.get(rule.category) ?? []), rule]);
    }
    return [...groups.entries()];
  });

  async function addRule(e: SubmitEvent) {
    e.preventDefault(); // formulár by inak znovu načítal stránku
    if (categoryId === null) return;
    try {
      const changed = await app.addRule(pattern, categoryId);
      message = `Pravidlo „${pattern.trim()}“ pridané · preradených transakcií: ${changed}`;
      pattern = "";
    } catch (err) {
      message = errorMessage(err);
    }
  }

  async function removeRule(id: number, label: string) {
    try {
      const changed = await app.deleteRule(id);
      message = `Pravidlo „${label}“ zmazané · zmenených transakcií: ${changed}`;
    } catch (err) {
      message = errorMessage(err);
    }
  }

  async function addCategory(e: SubmitEvent) {
    e.preventDefault();
    try {
      const category = await app.addCategory(newCategory);
      categoryId = category.id;
      message = `Kategória „${category.name}“ pridaná`;
      newCategory = "";
    } catch (err) {
      message = errorMessage(err);
    }
  }
</script>

<section class="card">
  <h2>Pravidlá kategorizácie</h2>
  <p class="secondary intro">
    Ak protistrana alebo popis transakcie obsahuje vzor (bez ohľadu na veľké písmená a diakritiku),
    transakcia dostane danú kategóriu. Pri viacerých zhodách vyhráva najdlhší vzor, takže „BOLT FOOD“
    má prednosť pred „BOLT“. Ručne zaradené transakcie pravidlá nemenia.
  </p>

  <div class="forms">
    <form onsubmit={addRule}>
      <input placeholder="Vzor, napr. LIDL" bind:value={pattern} required minlength="2" />
      <select bind:value={categoryId} required aria-label="Kategória">
        <option value={null} disabled>Vyber kategóriu</option>
        {#each app.categories as category (category.id)}
          <option value={category.id}>{category.name}</option>
        {/each}
      </select>
      <button class="primary" type="submit">Pridať pravidlo</button>
    </form>

    <form onsubmit={addCategory}>
      <input placeholder="Nová kategória" bind:value={newCategory} required />
      <button type="submit">Pridať kategóriu</button>
    </form>
  </div>

  {#if message}<p class="secondary">{message}</p>{/if}

  <div class="groups">
    {#each grouped as [category, rules] (category)}
      <div class="group">
        <h3>{category}</h3>
        <ul>
          {#each rules as rule (rule.id)}
            <li>
              <code>{rule.pattern}</code>
              <button class="link remove" onclick={() => removeRule(rule.id, rule.pattern)} aria-label="Zmazať pravidlo {rule.pattern}">
                ×
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {/each}
  </div>
</section>

<style>
  h2 {
    font-size: 15px;
  }

  .intro {
    max-width: 70ch;
  }

  .forms {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin: 12px 0;
  }

  form {
    display: flex;
    gap: 8px;
  }

  .groups {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 14px;
    margin-top: 14px;
  }

  h3 {
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px 2px 8px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface-raised);
  }

  code {
    font-size: 12.5px;
  }

  .remove {
    color: var(--text-muted);
    font-size: 16px;
    line-height: 1;
    padding: 0 4px;
  }

  .remove:hover {
    color: var(--critical);
  }
</style>
