<script lang="ts">
  // Import súborov: drag-and-drop z Findera alebo tlačidlo s dialógom.
  //
  // Prečo nie HTML5 `ondrop`? Tauri drop zachytí natívne a v udalosti
  // nám dá ABSOLÚTNE CESTY k súborom. Tie pošleme do Rustu, ktorý si súbor
  // prečíta sám cez std::fs – JavaScript obsah súboru vôbec nevidí.
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open } from "@tauri-apps/plugin-dialog";
  import { app } from "../state.svelte";
  import { errorMessage, type ImportOutcome } from "../api";

  let hovering = $state(false);
  // Promise uložená v stave → v šablóne ju rozbalí {#await}.
  let importing = $state<Promise<ImportOutcome[]> | null>(null);

  function startImport(paths: string[]) {
    const supported = paths.filter((p) => /\.(csv|xml|txt)$/i.test(p));
    if (supported.length === 0) return;
    importing = app.importFiles(supported);
  }

  // $effect beží po pripojení komponentu do DOM. Funkcia, ktorú vráti,
  // sa zavolá pri odpojení – tam zrušíme odber udalosti (inak "memory leak").
  $effect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          hovering = true;
        } else if (payload.type === "leave") {
          hovering = false;
        } else if (payload.type === "drop") {
          hovering = false;
          startImport(payload.paths);
        }
      })
      .then((fn) => {
        // Ak sa komponent odpojil skôr, než sa odber stihol vytvoriť.
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* mimo Tauri (napr. `npm run dev` v prehliadači) drop nefunguje */
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  });

  async function pickFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "Bankové výpisy", extensions: ["csv", "xml", "txt"] }],
    });
    if (selected) startImport(Array.isArray(selected) ? selected : [selected]);
  }
</script>

<section class="drop" class:hovering aria-label="Import výpisov">
  <div class="drop-text">
    <strong>{hovering ? "Pusť súbor sem" : "Pretiahni sem výpis z Georgea"}</strong>
    <span class="muted">CSV alebo XML (camt.053) · duplicitné platby sa preskočia</span>
  </div>
  <button onclick={pickFiles}>Vybrať súbor…</button>
</section>

{#if importing}
  <div class="result" aria-live="polite">
    {#await importing}
      <span class="muted">Importujem…</span>
    {:then outcomes}
      <ul>
        {#each outcomes as outcome (outcome.fileName)}
          <li>
            <strong>{outcome.fileName}</strong>
            {#if outcome.status === "ok"}
              — {outcome.format}: pridaných <strong>{outcome.imported}</strong>
              {#if outcome.duplicates > 0}<span class="muted">· duplicít {outcome.duplicates}</span>{/if}
              {#if outcome.skipped.length > 0}
                <details>
                  <summary class="error">Preskočených riadkov: {outcome.skipped.length}</summary>
                  <ul class="skipped">
                    {#each outcome.skipped as row (row.line)}
                      <li>riadok {row.line}: {row.message}</li>
                    {/each}
                  </ul>
                </details>
              {/if}
            {:else}
              — <span class="error">{outcome.error.message}</span>
            {/if}
          </li>
        {/each}
      </ul>
      <button class="link" onclick={() => (importing = null)}>Skryť</button>
    {:catch err}
      <span class="error">Import zlyhal: {errorMessage(err)}</span>
    {/await}
  </div>
{/if}

<style>
  .drop {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px;
    border: 1.5px dashed var(--axis);
    border-radius: var(--radius);
    background: var(--surface);
    transition:
      border-color 0.15s,
      background 0.15s;
  }

  .drop.hovering {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, var(--surface));
  }

  .drop-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .result {
    margin-top: 10px;
    padding: 10px 14px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .result ul {
    margin: 0 0 6px;
    padding-left: 18px;
  }

  .skipped {
    font-size: 13px;
    color: var(--text-secondary);
  }
</style>
