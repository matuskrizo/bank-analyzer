// Typy a obaly okolo Tauri príkazov.
//
// Typy tu musia zodpovedať Rust structom v src-tauri/src/model.rs
// (vďaka `#[serde(rename_all = "camelCase")]` sú polia v camelCase).
// Ak zmeníš struct v Ruste, zmeň aj typ tu – TypeScript to za teba
// neskontroluje. (Pokročilé: knižnice `specta`/`ts-rs` generujú typy z Rustu.)

import { invoke } from "@tauri-apps/api/core";

export interface Transaction {
  id: number;
  importId: number;
  date: string; // "2026-11-24"
  amountCents: number; // záporné = výdavok
  currency: string;
  counterparty: string | null;
  counterpartyAccount: string | null;
  description: string | null;
  variableSymbol: string | null;
  categoryId: number | null;
  category: string | null;
  categoryManual: boolean;
}

export interface Category {
  id: number;
  name: string;
}

export interface Rule {
  id: number;
  pattern: string;
  categoryId: number;
  category: string;
}

export interface ImportInfo {
  id: number;
  fileName: string;
  format: string;
  importedAt: string;
  rowCount: number;
}

export interface RowError {
  line: number;
  message: string;
}

/** Chyba z Rustu – pozri `impl Serialize for AppError` v error.rs. */
export interface AppError {
  kind: "io" | "csv" | "xml" | "db" | "format" | "input" | "internal";
  message: string;
}

/** Rust enum `ImportOutcome` s `#[serde(tag = "status")]`. */
export type ImportOutcome =
  | {
      status: "ok";
      fileName: string;
      format: string;
      imported: number;
      duplicates: number;
      skipped: RowError[];
    }
  | { status: "failed"; fileName: string; error: AppError };

/** Z ľubovoľnej chyby (AppError z Rustu, Error z JS, text) spraví čitateľnú správu. */
export function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) return String(err.message);
  return "Neznáma chyba";
}

// Názvy argumentov v camelCase – Tauri ich prevedie na snake_case v Ruste.
export const importFiles = (paths: string[]) => invoke<ImportOutcome[]>("import_files", { paths });
export const listTransactions = () => invoke<Transaction[]>("list_transactions");
export const listCategories = () => invoke<Category[]>("list_categories");
export const addCategory = (name: string) => invoke<Category>("add_category", { name });
export const listRules = () => invoke<Rule[]>("list_rules");
export const addRule = (pattern: string, categoryId: number) =>
  invoke<number>("add_rule", { pattern, categoryId });
export const deleteRule = (ruleId: number) => invoke<number>("delete_rule", { ruleId });
export const setCategory = (transactionId: number, categoryId: number | null) =>
  invoke<void>("set_category", { transactionId, categoryId });
export const listImports = () => invoke<ImportInfo[]>("list_imports");
export const deleteImport = (importId: number) => invoke<void>("delete_import", { importId });
