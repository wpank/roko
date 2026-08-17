export interface ConfigModelOption {
  key: string;
  name: string;
  slug: string;
  provider: string;
}

export type RawConfigModels = Record<string, { provider: string; slug: string }>;

export interface ProviderModelGroup {
  provider: string;
  kind: string;
  models: { key: string; name: string; slug: string }[];
}

export function groupConfigModels(
  models: RawConfigModels | undefined,
  providers: Record<string, { kind: string }> | undefined,
): ProviderModelGroup[] {
  if (!models) return [];
  const grouped = new Map<string, ProviderModelGroup>();
  for (const [key, model] of Object.entries(models)) {
    let group = grouped.get(model.provider);
    if (!group) {
      group = {
        provider: model.provider,
        kind: providers?.[model.provider]?.kind ?? 'unknown',
        models: [],
      };
      grouped.set(model.provider, group);
    }
    group.models.push({ key, name: key, slug: model.slug });
  }
  return Array.from(grouped.values());
}

export function flattenProviderModels(providers: ProviderModelGroup[]): ConfigModelOption[] {
  return providers.flatMap((provider) =>
    provider.models.map((model) => ({
      key: model.key,
      name: model.name,
      slug: model.slug,
      provider: provider.provider,
    })),
  );
}

export function findModelByKeyOrSlug(
  models: ConfigModelOption[],
  value: string,
): ConfigModelOption | undefined {
  return models.find((model) => model.key === value || model.slug === value);
}

export function resolveModelKey(models: ConfigModelOption[], value: string): string {
  return findModelByKeyOrSlug(models, value)?.key ?? value;
}

export function modelLabel(models: ConfigModelOption[], key: string): string {
  return models.find((model) => model.key === key)?.slug ?? key;
}

export function providerForModelKey(
  models: ConfigModelOption[],
  key: string,
): string | undefined {
  return models.find((model) => model.key === key)?.provider;
}

export function rawModelsToOptions(models: RawConfigModels | undefined): ConfigModelOption[] {
  if (!models) return [];
  return Object.entries(models).map(([key, model]) => ({
    key,
    name: key,
    slug: model.slug,
    provider: model.provider,
  }));
}
