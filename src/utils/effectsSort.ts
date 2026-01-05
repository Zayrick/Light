import type { EffectInfo } from "../types";

const OTHER_CATEGORY = "Other";

// Use a locale-aware collator so English/中文/数字都能更自然地排序。
// 如果运行环境不支持指定 locale，会自动回退到默认 locale。
const collator = new Intl.Collator("zh-CN", {
  numeric: true,
  sensitivity: "base",
});

export function getEffectCategory(effect: EffectInfo): string {
  const raw = effect.group?.trim();
  return raw && raw.length > 0 ? raw : OTHER_CATEGORY;
}

function compareCategory(a: string, b: string): number {
  if (a === b) return 0;
  if (a === OTHER_CATEGORY) return 1;
  if (b === OTHER_CATEGORY) return -1;
  return collator.compare(a, b);
}

export function sortEffectCategories(categories: string[]): string[] {
  // 保持输出去重且稳定。
  const uniq = Array.from(new Set(categories));
  return uniq.sort(compareCategory);
}

export function sortEffects(effects: EffectInfo[]): EffectInfo[] {
  return [...effects].sort((a, b) => {
    const ca = getEffectCategory(a);
    const cb = getEffectCategory(b);

    const byCategory = compareCategory(ca, cb);
    if (byCategory !== 0) return byCategory;

    const byName = collator.compare(a.name, b.name);
    if (byName !== 0) return byName;

    // 最终兜底：用 id 保证完全确定性
    return collator.compare(a.id, b.id);
  });
}
