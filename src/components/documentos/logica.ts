// Pure rules of the Currículos tab, tested with node:test (tests/front/). No React, no Tauri.
import type { Documento, Tag, TipoTag } from "../../types";

export type Filtro = Partial<Record<Exclude<TipoTag, "livre">, string>>;

export function filtrar(docs: Documento[], f: Filtro): Documento[] {
  const pedidos = Object.entries(f).filter(([, v]) => v) as [TipoTag, string][];
  return docs.filter((d) => pedidos.every(([tipo, valor]) => d.tags.some((t) => t.tipo === tipo && t.valor === valor)));
}

export function valoresDeTag(docs: Documento[], tipo: TipoTag): string[] {
  return [...new Set(docs.flatMap((d) => d.tags.filter((t) => t.tipo === tipo).map((t) => t.valor)))].sort();
}

export function ehPreferido(d: Documento, varianteId: string | null): boolean {
  return varianteId === null ? d.preferido_global : d.preferido_variantes.includes(varianteId);
}

export function avisoAoApagar(d: Documento): "preferido_global" | "preferido_variante" | null {
  if (d.preferido_global) return "preferido_global";
  if (d.preferido_variantes.length > 0) return "preferido_variante";
  return null;
}

export function preferidoDaVariante(docs: Documento[], varianteId: string): Documento | null {
  return docs.find((d) => d.preferido_variantes.includes(varianteId)) ?? null;
}

export function comTag(tags: Tag[], nova: Tag): Tag[] {
  const valor = nova.valor.trim();
  if (!valor || tags.some((t) => t.tipo === nova.tipo && t.valor === valor)) return tags;
  return [...tags, { tipo: nova.tipo, valor }];
}

export function semTag(tags: Tag[], fora: Tag): Tag[] {
  return tags.filter((t) => !(t.tipo === fora.tipo && t.valor === fora.valor));
}
