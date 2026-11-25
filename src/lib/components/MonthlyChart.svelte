<script lang="ts">
  // Príjmy vs. výdavky po mesiacoch – zoskupené stĺpce v SVG.
  //
  // Graf kreslíme sami: z dát vypočítame súradnice ($derived) a Svelte
  // vykreslí <path> pre každý stĺpec. Šírku zistíme cez bind:clientWidth,
  // takže graf sa prispôsobí oknu.
  import { app } from "../state.svelte";
  import { money, moneyCompact, month } from "../format";

  const HEIGHT = 240;
  const M = { top: 12, right: 8, bottom: 28, left: 64 };
  const BAR_GAP = 2; // medzera medzi príjmom a výdavkom v skupine

  let width = $state(600);
  let hovered = $state<number | null>(null);

  let data = $derived(app.byMonth);
  let innerW = $derived(Math.max(0, width - M.left - M.right));
  let innerH = HEIGHT - M.top - M.bottom;

  /** "Pekné" maximum osi a kroky: 1, 2 alebo 5 × 10^n. */
  let scale = $derived.by(() => {
    const max = Math.max(1, ...data.flatMap((d) => [d.income, d.expense]));
    const raw = max / 4;
    const pow = 10 ** Math.floor(Math.log10(raw));
    const step = [1, 2, 5, 10].map((f) => f * pow).find((s) => s >= raw)!;
    const top = Math.ceil(max / step) * step;
    const ticks = Array.from({ length: Math.round(top / step) + 1 }, (_, i) => i * step);
    return { top, ticks };
  });

  const y = (cents: number) => M.top + innerH - (cents / scale.top) * innerH;

  let band = $derived(data.length > 0 ? innerW / data.length : 0);
  let barW = $derived(Math.max(2, Math.min(28, (band * 0.6 - BAR_GAP) / 2)));

  /** Obdĺžnik so zaoblenými hornými rohmi (4 px), dolná hrana na osi. */
  function barPath(x: number, value: number): string {
    const top = y(value);
    const h = M.top + innerH - top;
    if (h <= 0) return "";
    const r = Math.min(4, barW / 2, h);
    const base = M.top + innerH;
    return `M${x},${base} V${top + r} Q${x},${top} ${x + r},${top} H${x + barW - r} Q${x + barW},${top} ${x + barW},${top + r} V${base} Z`;
  }

  const groupX = (i: number) => M.left + i * band + band / 2;
</script>

<section class="card">
  <div class="head">
    <h2>Príjmy a výdavky po mesiacoch</h2>
    <div class="legend" aria-hidden="true">
      <span><i style:background="var(--series-1)"></i>Príjmy</span>
      <span><i style:background="var(--series-2)"></i>Výdavky</span>
    </div>
  </div>

  {#if data.length === 0}
    <p class="muted empty">Žiadne dáta vo vybranom období.</p>
  {:else}
    <div class="chart" bind:clientWidth={width}>
      <svg {width} height={HEIGHT} role="img" aria-label="Stĺpcový graf príjmov a výdavkov po mesiacoch">
        <!-- mriežka a popisy osi Y -->
        {#each scale.ticks as tick (tick)}
          <line
            x1={M.left}
            x2={width - M.right}
            y1={y(tick)}
            y2={y(tick)}
            stroke={tick === 0 ? "var(--axis)" : "var(--grid)"}
          />
          <text x={M.left - 8} y={y(tick)} dy="0.32em" text-anchor="end" class="axis">
            {moneyCompact(tick)}
          </text>
        {/each}

        {#each data as d, i (d.month)}
          {@const cx = groupX(i)}
          <g class:dim={hovered !== null && hovered !== i}>
            <path d={barPath(cx - barW - BAR_GAP / 2, d.income)} fill="var(--series-1)" />
            <path d={barPath(cx + BAR_GAP / 2, d.expense)} fill="var(--series-2)" />
          </g>
          <text x={cx} y={HEIGHT - 8} text-anchor="middle" class="axis">{month(d.month)}</text>

          <!-- neviditeľná plocha pre hover – väčšia než samotné stĺpce -->
          <rect
            x={M.left + i * band}
            y={M.top}
            width={band}
            height={innerH}
            fill="transparent"
            tabindex="0"
            role="button"
            aria-label="{month(d.month)}: príjmy {money(d.income)}, výdavky {money(d.expense)}"
            onpointerenter={() => (hovered = i)}
            onpointerleave={() => (hovered = null)}
            onfocus={() => (hovered = i)}
            onblur={() => (hovered = null)}
          />
        {/each}
      </svg>

      {#if hovered !== null && data[hovered]}
        {@const d = data[hovered]}
        <div
          class="tip"
          role="tooltip"
          style:left="{Math.min(Math.max(groupX(hovered), 90), width - 90)}px"
          style:top="{Math.max(y(Math.max(d.income, d.expense)) - 8, 0)}px"
        >
          <div class="tip-title">{month(d.month)}</div>
          <div class="tip-row"><i style:background="var(--series-1)"></i><strong>{money(d.income)}</strong> príjmy</div>
          <div class="tip-row"><i style:background="var(--series-2)"></i><strong>{money(d.expense)}</strong> výdavky</div>
          <div class="tip-row bal">
            Bilancia <strong class:positive={d.income > d.expense}>{money(d.income - d.expense)}</strong>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 8px;
  }

  h2 {
    font-size: 15px;
  }

  .legend {
    display: flex;
    gap: 14px;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .legend span {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .legend i,
  .tip-row i {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }

  .tip-row i {
    width: 12px;
    height: 2px;
  }

  .empty {
    margin: 24px 0;
  }

  .chart {
    position: relative;
  }

  svg {
    display: block;
    overflow: visible;
  }

  .axis {
    fill: var(--text-muted);
    font-size: 12px;
  }

  g {
    transition: opacity 0.12s;
  }

  g.dim {
    opacity: 0.45;
  }

  rect:focus-visible {
    outline: none;
    stroke: var(--accent);
    stroke-width: 2px;
  }

  .tip {
    position: absolute;
    transform: translate(-50%, -100%);
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.14);
    font-size: 13px;
    white-space: nowrap;
    pointer-events: none;
  }

  .tip-title {
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .tip-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-variant-numeric: tabular-nums;
  }

  .bal {
    margin-top: 4px;
    padding-top: 4px;
    border-top: 1px solid var(--border);
    color: var(--text-secondary);
  }
</style>
