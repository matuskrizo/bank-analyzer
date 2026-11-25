// Formátovanie súm a dátumov pre slovenské prostredie.

const eur = new Intl.NumberFormat("sk-SK", { style: "currency", currency: "EUR" });
const eurCompact = new Intl.NumberFormat("sk-SK", {
  style: "currency",
  currency: "EUR",
  notation: "compact",
  maximumFractionDigits: 1,
});

/** -4731 → "-47,31 €" */
export const money = (cents: number) => eur.format(cents / 100);

/** 185000 → "1,9 tis. €" – na osi grafu. */
export const moneyCompact = (cents: number) => eurCompact.format(cents / 100);

/** "2026-11-24" → "24. 11. 2026" */
export function date(iso: string): string {
  const [y, m, d] = iso.split("-");
  return `${Number(d)}. ${Number(m)}. ${y}`;
}

/** "2026-11" → "nov 2026" */
export function month(key: string): string {
  const [y, m] = key.split("-").map(Number);
  return new Intl.DateTimeFormat("sk-SK", { month: "short", year: "numeric" }).format(
    new Date(y, m - 1, 1),
  );
}

/** Date → "2026-11-24" v lokálnom čase (toISOString by posunul dátum podľa UTC). */
export function isoDate(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Text na vyhľadávanie: malé písmená, bez diakritiky ("Kaviareň" → "kaviaren"). */
export function fold(s: string): string {
  return s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
}

export interface DatePreset {
  label: string;
  from: string;
  to: string;
}

/** Predvoľby obdobia pre filter – vždy počítané od dnešného dňa. */
export function datePresets(today = new Date()): DatePreset[] {
  const y = today.getFullYear();
  const m = today.getMonth();
  return [
    { label: "Všetko", from: "", to: "" },
    { label: "Tento mesiac", from: isoDate(new Date(y, m, 1)), to: isoDate(today) },
    {
      label: "Minulý mesiac",
      from: isoDate(new Date(y, m - 1, 1)),
      to: isoDate(new Date(y, m, 0)), // deň 0 = posledný deň predošlého mesiaca
    },
    { label: "Posledné 3 mesiace", from: isoDate(new Date(y, m - 2, 1)), to: isoDate(today) },
    { label: "Tento rok", from: `${y}-01-01`, to: isoDate(today) },
  ];
}
