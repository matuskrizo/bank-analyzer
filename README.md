# Analyzátor bankových výpisov

Desktopová aplikácia (Tauri 2 + Svelte 5) na učenie sa Rustu. Do okna pretiahneš výpis
z Georgea (Slovenská sporiteľňa) a aplikácia transakcie uloží do lokálnej SQLite databázy,
roztriedi ich do kategórií a zobrazí štatistiky a grafy.

Projekt nemá žiadny HTTP server, CORS ani prihlasovanie. Rust a Svelte sa rozprávajú
priamo cez Tauri príkazy (IPC).

## Spustenie

Potrebuješ:

- **Rust** (`rustup`, stačí stabilná verzia; projekt používa edíciu 2024)
- **Node.js** 20 alebo novší
- na macOS **Xcode Command Line Tools** (`xcode-select --install`)

```bash
npm install            # nainštaluje frontend závislosti (aj Tauri CLI)
npm run tauri dev      # spustí Vite + skompiluje Rust + otvorí okno
```

Prvá kompilácia Rustu trvá pár minút, ďalšie už len sekundy. Zmeny v `.svelte` súboroch
sa prejavia okamžite, po zmene v `.rs` súboroch sa aplikácia sama reštartuje.

Na skúšku pretiahni do okna súbory z priečinka `samples/`. Druhý import toho istého súboru
pridá 0 transakcií, lebo duplicity sa preskočia.

Ďalšie príkazy:

```bash
cd src-tauri && cargo test     # 24 testov: parser, kategorizácia, databáza
cd src-tauri && cargo clippy   # rady, ako písať idiomatickejší Rust
npm run check                  # kontrola typov v Svelte/TypeScript
npm run tauri build            # hotová .app / .dmg v src-tauri/target/release/bundle
```

Databáza je na macOS v `~/Library/Application Support/sk.levelup.bank-analyzer/transakcie.db`.
Ak ju zmažeš, aplikácia začne odznova (s predvolenými kategóriami a pravidlami).

## Tvoj výpis zo SLSP

V Georgeovi exportuj pohyby na účte ako **CSV**. Ak máš k dispozícii výpis v **XML
(camt.053)**, funguje aj ten. Parser nehľadá stĺpce podľa poradia, ale podľa názvu.
Pri exporte z Georgea použije „Dátum splatnosti“, „Suma“, „Mena“, „Partner“, „IBAN partnera“,
„Variabilný symbol“ a do popisu spojí „Popis transakcie“ s „Typom transakcie“ (napr. „Platba kartou“,
„Výber kartou“). Typ transakcie pomáha kategorizácii: výbery, poplatky, investície a splátky úverov
sa tak zaradia aj vtedy, keď chýba partner. Zvládne oddeľovač `;`, `,` aj tabulátor, kódovanie UTF-8 aj Windows-1250,
desatinnú čiarku a aj riadky s informáciami o účte nad hlavičkou.

Ak import hlási *„nenašiel som hlavičku so stĺpcami dátumu a sumy“* alebo nejaký stĺpec
chýba, otvor `src-tauri/src/parser/george_csv.rs` a doplň názov stĺpca (bez diakritiky,
malými písmenami) do príslušného zoznamu `DATE`, `AMOUNT`, `COUNTERPARTY`… Potom pridaj
test s anonymizovanou hlavičkou tvojho exportu. To je dobré prvé cvičenie.

## Ako je projekt poskladaný

```
src-tauri/src/            ← Rust
  lib.rs                  štart aplikácie: plugin, databáza do State, registrácia príkazov
  commands.rs             #[tauri::command] funkcie volané z JS (tenká vrstva)
  error.rs                AppError (thiserror) + ručný Serialize → { kind, message }
  model.rs                structy s #[serde(rename_all = "camelCase")]
  parser/
    mod.rs                rozpoznanie formátu podľa obsahu
    text.rs               Windows-1250 → UTF-8, porovnávanie bez diakritiky
    amount.rs             "-1 234,56" → -123456 centov (bez f64!)
    date.rs               "24. 11. 2026" → NaiveDate
    george_csv.rs         hľadanie hlavičky, mapovanie stĺpcov podľa názvu
    camt053.rs            ISO 20022 XML cez serde + quick-xml
  categorize.rs           pravidlá „vzor → kategória“, najdlhší vzor vyhráva
  db.rs                   rusqlite: migrácie, import v transakcii, deduplikácia
src/                      ← Svelte 5
  lib/api.ts              TypeScript typy + obaly okolo invoke()
  lib/state.svelte.ts     zdieľaný stav: $state, $derived (filtre, súčty, grafy)
  lib/components/         DropZone, Filters, StatTiles, MonthlyChart, CategoryChart,
                          TransactionTable, RulesPanel, ImportsPanel
samples/                  ukážkové výpisy (fiktívne dáta) – používajú ich aj testy
```

Tok dát pri importe:

```
drag & drop ──(cesty k súborom)──▶ invoke("import_files")
   ──▶ parser::parse_file   std::fs::read → dekódovanie → CSV/XML → Vec<ParsedTransaction>
   ──▶ db::import           kategorizácia + INSERT OR IGNORE (hash) v jednej SQL transakcii
   ──▶ ImportOutcome         { status: "ok", imported, duplicates, skipped } → {#await} v Svelte
   ──▶ app.refresh()         nové dáta do $state → $derived prepočíta tabuľku aj grafy
```

## Čo sa kde naučíš

**Rust**

| Koncept | Kde |
|---|---|
| `Result`, operátor `?`, `From` konverzie chýb | `parser/mod.rs`, `error.rs` |
| `thiserror` a ručná implementácia traitu `Serialize` | `error.rs` |
| `serde` derive, premenovanie polí, tagované enumy | `model.rs`, `commands.rs` (`ImportOutcome`) |
| Parsovanie XML do structov, `Option<Box<T>>` | `parser/camt053.rs` |
| Iterátory: `filter`, `find_map`, `flat_map`, `max_by_key` | `george_csv.rs`, `categorize.rs` |
| Closures ako parametre (`impl Fn(usize) -> &str`) | `george_csv.rs` → `parse_row` |
| Let chains (`if let … && …`, edícia 2024) | `george_csv.rs` |
| Životnosť výpožičky (`stmt` musí zaniknúť pred `commit`) | `db.rs` → `import` |
| Zdieľaný stav: `Mutex<Connection>` v `tauri::State` | `commands.rs` |
| `async` príkazy, aby UI nezamŕzalo | `commands.rs` → `import_files` |
| Unit testy, `include_str!`, databáza v pamäti | `#[cfg(test)]` v každom module |

**Svelte 5**

| Koncept | Kde |
|---|---|
| `$state` v triede v `.svelte.ts` (zdieľaný stav) | `lib/state.svelte.ts` |
| `$derived` / `$derived.by` (filtre, súčty, dáta grafov) | `lib/state.svelte.ts`, grafy |
| `$effect` s upratovaním (odber Tauri udalosti) | `DropZone.svelte` |
| `{#await promise}` pre stav načítavania | `DropZone.svelte` |
| `bind:value`, `bind:clientWidth` | `Filters.svelte`, `MonthlyChart.svelte` |
| Kľúčované `{#each … (id)}`, `{@const}` | `TransactionTable.svelte`, `MonthlyChart.svelte` |
| Graf v čistom SVG bez knižnice | `MonthlyChart.svelte` |

## Úlohy na precvičenie

Zoradené zhruba od najľahšej:

1. **Tvoj export.** Pridaj do testov v `george_csv.rs` anonymizovanú hlavičku a dva riadky
   z tvojho skutočného CSV zo SLSP.
2. **Vlastné pravidlá.** V karte Pravidlá si pridaj obchody, kde nakupuješ. Potom zisti
   v `categorize.rs`, prečo ručne zaradené transakcie ostanú nezmenené.
3. **Prevody medzi vlastnými účtami.** Pridaj kategóriu „Prevod“, ktorá sa nezapočíta
   do príjmov ani výdavkov (`state.svelte.ts` → `totals`).
4. **Filter v SQL.** Napíš príkaz `get_transactions(filter)`, ktorý filtruje cez `WHERE`
   v Ruste, a porovnaj ho s filtrovaním cez `$derived`. Kedy sa oplatí ktoré?
5. **Export do CSV.** Nový príkaz uloží odfiltrované transakcie do súboru. Cestu vyber
   cez `save()` z `@tauri-apps/plugin-dialog` a zapíš pomocou `csv::Writer`.
6. **Rozpočty.** Tabuľka `budgets (category_id, monthly_limit_cents)` (napíš
   `migration_2`!) a v grafe kategórií zobraz čerpanie rozpočtu.
7. **Typy automaticky.** Vygeneruj TypeScript typy z Rust structov cez `specta` alebo
   `ts-rs` namiesto ručného `api.ts`.
8. **Graf zostatku.** Čiarový graf priebežného zostatku v čase (kumulatívna suma
   cez `$derived`).
