// Zdieľaný stav aplikácie pomocou Svelte 5 rún.
//
// Súbor má príponu `.svelte.ts`, preto v ňom fungujú runy ($state, $derived)
// aj mimo komponentov. Vytvoríme jednu inštanciu triedy a importujeme ju
// v komponentoch – každá zmena `$state` poľa automaticky prekreslí všetko,
// čo od neho závisí.
//
//   $state    – reaktívna hodnota (pole/objekt je "hlboko" reaktívny proxy)
//   $derived  – hodnota vypočítaná z iných; prepočíta sa sama, keď sa zmenia
//
// Filtrovanie robíme tu na frontende – pre tisíce transakcií je to rýchle
// a filtre reagujú okamžite bez volania Rustu.

import * as api from "./api";
import { fold } from "./format";

export type Direction = "all" | "in" | "out";
/** "all" = všetky, "none" = nezaradené, číslo = id kategórie */
export type CategoryFilter = "all" | "none" | number;

export const UNCATEGORIZED = "Nezaradené";

export interface CategoryTotal {
  name: string;
  categoryId: number | null;
  cents: number; // kladné číslo – suma výdavkov
  count: number;
}

export interface MonthTotal {
  month: string; // "2026-11"
  income: number;
  expense: number; // kladné číslo
}

class AppData {
  transactions = $state<api.Transaction[]>([]);
  categories = $state<api.Category[]>([]);
  rules = $state<api.Rule[]>([]);
  imports = $state<api.ImportInfo[]>([]);
  loading = $state(true);
  loadError = $state<string | null>(null);

  filters = $state({
    from: "",
    to: "",
    category: "all" as CategoryFilter,
    direction: "all" as Direction,
    search: "",
  });

  /** Transakcie po aplikovaní všetkých filtrov. */
  filtered = $derived.by(() => {
    const { from, to, category, direction, search } = this.filters;
    const needle = fold(search.trim());
    return this.transactions.filter((t) => {
      // ISO dátumy sa dajú porovnávať ako reťazce: "2026-10-31" < "2026-11-01"
      if (from && t.date < from) return false;
      if (to && t.date > to) return false;
      if (direction === "in" && t.amountCents <= 0) return false;
      if (direction === "out" && t.amountCents >= 0) return false;
      if (category === "none" && t.categoryId !== null) return false;
      if (typeof category === "number" && t.categoryId !== category) return false;
      if (needle) {
        const haystack = fold(
          [t.counterparty, t.description, t.variableSymbol, t.category].filter(Boolean).join(" "),
        );
        if (!haystack.includes(needle)) return false;
      }
      return true;
    });
  });

  totals = $derived.by(() => {
    let income = 0;
    let expense = 0;
    for (const t of this.filtered) {
      if (t.amountCents > 0) income += t.amountCents;
      else expense -= t.amountCents;
    }
    return { income, expense, balance: income - expense, count: this.filtered.length };
  });

  /** Výdavky podľa kategórie, od najväčšej. */
  byCategory = $derived.by((): CategoryTotal[] => {
    const map = new Map<string, CategoryTotal>();
    for (const t of this.filtered) {
      if (t.amountCents >= 0) continue;
      const name = t.category ?? UNCATEGORIZED;
      const entry = map.get(name) ?? { name, categoryId: t.categoryId, cents: 0, count: 0 };
      entry.cents -= t.amountCents;
      entry.count += 1;
      map.set(name, entry);
    }
    return [...map.values()].sort((a, b) => b.cents - a.cents);
  });

  /** Príjmy a výdavky po mesiacoch, chronologicky. */
  byMonth = $derived.by((): MonthTotal[] => {
    const map = new Map<string, MonthTotal>();
    for (const t of this.filtered) {
      const month = t.date.slice(0, 7);
      const entry = map.get(month) ?? { month, income: 0, expense: 0 };
      if (t.amountCents > 0) entry.income += t.amountCents;
      else entry.expense -= t.amountCents;
      map.set(month, entry);
    }
    return [...map.values()].sort((a, b) => a.month.localeCompare(b.month));
  });

  hasActiveFilters = $derived(
    this.filters.from !== "" ||
      this.filters.to !== "" ||
      this.filters.category !== "all" ||
      this.filters.direction !== "all" ||
      this.filters.search !== "",
  );

  resetFilters() {
    Object.assign(this.filters, { from: "", to: "", category: "all", direction: "all", search: "" });
  }

  /** Načíta všetko z databázy. Promise.all → 4 príkazy bežia naraz. */
  async refresh() {
    try {
      const [transactions, categories, rules, imports] = await Promise.all([
        api.listTransactions(),
        api.listCategories(),
        api.listRules(),
        api.listImports(),
      ]);
      this.transactions = transactions;
      this.categories = categories;
      this.rules = rules;
      this.imports = imports;
      this.loadError = null;
    } catch (err) {
      this.loadError = api.errorMessage(err);
    } finally {
      this.loading = false;
    }
  }

  async importFiles(paths: string[]) {
    const outcomes = await api.importFiles(paths);
    await this.refresh();
    return outcomes;
  }

  async setCategory(transactionId: number, categoryId: number | null) {
    await api.setCategory(transactionId, categoryId);
    await this.refresh();
  }

  async addRule(pattern: string, categoryId: number) {
    const changed = await api.addRule(pattern, categoryId);
    await this.refresh();
    return changed;
  }

  async deleteRule(ruleId: number) {
    const changed = await api.deleteRule(ruleId);
    await this.refresh();
    return changed;
  }

  async addCategory(name: string) {
    const category = await api.addCategory(name);
    await this.refresh();
    return category;
  }

  async deleteImport(importId: number) {
    await api.deleteImport(importId);
    await this.refresh();
  }
}

export const app = new AppData();
