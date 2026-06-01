<script lang="ts">
  import type { TestResult } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    results = [],
    onUse,
    onDetails
  }: {
    results?: TestResult[];
    onUse: (result: TestResult) => void;
    onDetails: (result: TestResult) => void;
  } = $props();

  const pageSize = 5;
  let page = $state(1);
  const totalPages = $derived(Math.max(1, Math.ceil(results.length / pageSize)));
  const pageItems = $derived(results.slice((page - 1) * pageSize, page * pageSize));
  const pages = $derived.by(() => {
    if (totalPages <= 7) return Array.from({ length: totalPages }, (_, index) => index + 1);

    const values = new Set([1, totalPages, page - 1, page, page + 1]);
    const sorted = Array.from(values)
      .filter((value) => value >= 1 && value <= totalPages)
      .sort((left, right) => left - right);
    const compact: number[] = [];
    for (const value of sorted) {
      const previous = compact[compact.length - 1];
      if (previous && value - previous > 1) compact.push(0);
      compact.push(value);
    }
    return compact;
  });

  $effect(() => {
    if (page > totalPages) page = totalPages;
  });
</script>

<div class="recommendations">
  {#if results.length === 0}
    <p class="muted-text">{$t("test.recommendationsEmpty")}</p>
  {:else}
    <div class="recommendations-head">
      <span>{$t("test.presetsTested", { count: results.length })}</span>
      <span>{$t("common.page")} {page}/{totalPages}</span>
    </div>

    {#each pageItems as result}
      <article class="recommendation-row">
        <div class="score-pill">{result.score}</div>
        <div class="recommendation-main">
          <strong>{result.presetName}</strong>
          <span>{$t("test.passed", { ok: result.ok, total: result.total })}</span>
        </div>
        <button class="secondary-button" type="button" onclick={() => onDetails(result)}>{$t("common.details")}</button>
        <button class="secondary-button" type="button" onclick={() => onUse(result)}>{$t("common.use")}</button>
      </article>
    {/each}

    <div class="pager">
      {#if page < totalPages}
        <button class="secondary-button more-button" type="button" onclick={() => (page += 1)}>{$t("common.more")}</button>
      {/if}
      <div class="page-numbers" aria-label={$t("test.resultPages")}>
        {#each pages as value}
          {#if value === 0}
            <span aria-hidden="true">...</span>
          {:else}
            <button
              type="button"
              class:active={value === page}
              onclick={() => (page = value)}
              aria-label={`${$t("common.page")} ${value}`}
            >
              {value}
            </button>
          {/if}
        {/each}
      </div>
    </div>
  {/if}
</div>
