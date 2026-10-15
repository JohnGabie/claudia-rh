import { test } from "node:test";
import assert from "node:assert/strict";
import { dataLocal, diasDesde, filtrar, ordenar, valoresDe, type Site } from "../../src/components/sites/logica.ts";

const base: Omit<Site, "chave" | "nome"> = {
  url_login: "https://x.com/", dominios: ["x.com"], dominios_ats: [], caminho_ats: null, tipo: "geral", regiao: "GLOBAL", pais: null,
  papel: "candidatura", regra_login: "manual", ia_pode_usar: false, embutida: true, padrao_atual: true,
  estado: "desconhecido", verificada_em: null,
};
const s = (chave: string, nome: string, extra: Partial<Site> = {}): Site => ({ ...base, chave, nome, ...extra });
const sites = [
  s("orsted", "Ørsted", { dominios: ["orsted.com"], tipo: "empresa", regiao: "EU", pais: "DK" }),
  s("zurich", "Zürich Jobs", { dominios: ["zurich.ch"], regiao: "EU", pais: "CH", estado: "logada" }),
  s("jobindex", "Jobindex", { dominios: ["jobindex.dk"], regiao: "EU", pais: "DK", estado: "precisa_login" }),
  s("vagas", "Vagas.com", { dominios: ["vagas.com.br"], regiao: "SA", pais: "BR" }),
  s("linkedin", "LinkedIn", { dominios: ["linkedin.com"] }),
];
const nenhum = { busca: "", tipo: "", regiao: "", pais: "" };

test("empty filters keep everything", () => assert.equal(filtrar(sites, nenhum).length, 5));

test("search ignores case and accents and also looks at the domain", () => {
  assert.deepEqual(filtrar(sites, { ...nenhum, busca: "zurich" }).map((x) => x.chave), ["zurich"]);
  assert.deepEqual(filtrar(sites, { ...nenhum, busca: "ØRSTED" }).map((x) => x.chave), ["orsted"]);
  assert.deepEqual(filtrar(sites, { ...nenhum, busca: "jobindex.dk" }).map((x) => x.chave), ["jobindex"]);
  // only the name carries the accent here (domain is zurich.ch), so this fails without accent folding
  assert.deepEqual(filtrar(sites, { ...nenhum, busca: "zurich jobs" }).map((x) => x.chave), ["zurich"]);
  assert.deepEqual(filtrar(sites, { ...nenhum, busca: "zürich" }).map((x) => x.chave), ["zurich"]);
});

test("search, type, region and country combine", () => {
  assert.deepEqual(filtrar(sites, { busca: "", tipo: "", regiao: "EU", pais: "DK" }).map((x) => x.chave).sort(), ["jobindex", "orsted"]);
  assert.deepEqual(filtrar(sites, { busca: "o", tipo: "empresa", regiao: "EU", pais: "DK" }).map((x) => x.chave), ["orsted"]);
});

test("a filter that matches nothing gives an empty list, not an error", () => {
  assert.deepEqual(filtrar(sites, { ...nenhum, busca: "nada-disso" }), []);
  assert.deepEqual(filtrar([], nenhum), []);
});

test("sorting by name ignores accents and is stable on ties", () => {
  assert.deepEqual(ordenar(sites, { campo: "nome", desc: false }).map((x) => x.chave), ["jobindex", "linkedin", "orsted", "vagas", "zurich"]);
  assert.deepEqual(ordenar(sites, { campo: "nome", desc: true }).map((x) => x.chave), ["zurich", "vagas", "orsted", "linkedin", "jobindex"]);
  const iguais = [s("b", "Mesmo"), s("a", "Mesmo")];
  assert.deepEqual(ordenar(iguais, { campo: "nome", desc: false }).map((x) => x.chave), ["a", "b"]);
});

test("sorting by login state puts logged-in first, then unknown, then needs-login", () => {
  assert.deepEqual(ordenar(sites, { campo: "estado", desc: false }).map((x) => x.estado),
    ["logada", "desconhecido", "desconhecido", "desconhecido", "precisa_login"]);
});

test("sorting does not change the input", () => {
  const copia = [...sites];
  ordenar(sites, { campo: "pais", desc: false });
  assert.deepEqual(sites, copia);
});

test("the filter options are the distinct values present, sorted, without empties", () => {
  assert.deepEqual(valoresDe(sites, "regiao"), ["EU", "GLOBAL", "SA"]);
  assert.deepEqual(valoresDe(sites, "pais"), ["BR", "CH", "DK"]);
});

test("days since the confirmation read the SQLite UTC timestamp", () => {
  const agora = new Date("2026-10-14T12:00:00Z");
  assert.equal(diasDesde("2026-10-11 12:00:00", agora), 3);
  assert.equal(diasDesde("2026-10-14 11:00:00", agora), 0);
  assert.equal(diasDesde(null, agora), null);
  assert.equal(diasDesde("lixo", agora), null);
});

test("sorting by country puts sites without a country last in both directions", () => {
  const asc = ordenar(sites, { campo: "pais", desc: false }).map((x) => x.pais);
  assert.deepEqual(asc.filter((p) => p !== null), ["BR", "CH", "DK", "DK"]);
  assert.deepEqual(asc.slice(-1), [null]);
  assert.equal(asc.length, 5);
  const desc = ordenar(sites, { campo: "pais", desc: true }).map((x) => x.pais);
  assert.deepEqual(desc.filter((p) => p !== null), ["DK", "DK", "CH", "BR"]);
  assert.deepEqual(desc.slice(-1), [null]);
  assert.equal(ordenar(sites, { campo: "pais", desc: true })[4].chave, "linkedin");
  assert.equal(ordenar(sites, { campo: "pais", desc: false })[4].chave, "linkedin");
});

test("days since accepts an ISO string that already has a zone", () => {
  const agora = new Date("2026-10-14T12:00:00Z");
  assert.equal(diasDesde("2026-10-11T12:00:00Z", agora), 3);
  assert.equal(diasDesde("2026-10-11T14:00:00+02:00", agora), 3);
  assert.equal(diasDesde("2026-10-11 12:00:00", agora), 3);
});

test("search folds letters that NFD does not decompose", () => {
  const lista = [
    s("orsted", "Ørsted Careers", { dominios: ["orsted.com"] }),
    s("strasse", "Straße Jobs", { dominios: ["strasse.de"] }),
    s("other", "Other", { dominios: ["other.com"] }),
  ];
  const busca = (q: string) => filtrar(lista, { busca: q, tipo: "", regiao: "", pais: "" }).map((x) => x.chave);
  assert.deepEqual(busca("orsted"), ["orsted"]);
  assert.deepEqual(busca("orsted careers"), ["orsted"]);
  assert.deepEqual(busca("ØRSTED"), ["orsted"]);
  assert.deepEqual(busca("strasse"), ["strasse"]);
});

test("name sort is numeric-aware", () => {
  const lista = [s("a", "Número 29"), s("b", "Número 3"), s("c", "Número 100")];
  assert.deepEqual(ordenar(lista, { campo: "nome", desc: false }).map((x) => x.nome), ["Número 3", "Número 29", "Número 100"]);
});

test("dataLocal formats a UTC timestamp and never yields Invalid Date", () => {
  assert.match(dataLocal("2026-10-11 12:00:00", "pt") ?? "", /^11\/10\/2026$/);
  assert.equal(dataLocal(null, "pt"), null);
  assert.equal(dataLocal("lixo", "en"), null);
});
