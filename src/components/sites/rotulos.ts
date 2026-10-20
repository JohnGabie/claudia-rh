// Translated label of a catalog value (type, region), falling back to the raw value when unknown.
export const rotulo = (mapa: object, valor: string): string => (mapa as Record<string, string>)[valor] ?? valor;
