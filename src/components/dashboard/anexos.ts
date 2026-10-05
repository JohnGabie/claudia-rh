// Pure helpers for "where the document shows up" (spec 2.2, P5/P6). No React, no Tauri.

export interface AnexoAtual {
  tipo: "curriculo" | "carta";
  documento_id: number | null;
  nome: string | null;
  formato: string | null;
}

// A deleted document comes back as an item whose id, name and format are all null.
export const rotuloAnexo = (a: AnexoAtual, apagado: string): string => {
  if (a.documento_id === null || a.nome === null) return apagado;
  return a.formato ? `${a.nome} (${a.formato.toUpperCase()})` : a.nome;
};

// History column: the name of the document sent, or a dash (no attachment, or the document was deleted).
export const rotuloDocumentoCandidatura = (c: { documento_id: number | null; documento_nome: string | null }): string =>
  c.documento_id === null || c.documento_nome === null ? "—" : c.documento_nome;

// Pendência categories that mean "the library has no document of that kind" (P6).
export const ABA_DA_PENDENCIA: Record<string, "curriculo" | "carta"> = {
  sem_curriculo: "curriculo",
  sem_carta: "carta",
};

export const ehSemDocumento = (categoria: string): boolean =>
  Object.keys(ABA_DA_PENDENCIA).some((k) => categoria.toLowerCase().includes(k));

export interface UploadLog {
  id: number;
  nome: string;
  formato: string;
  bytes: number;
  sha: string;
}

// browser_upload rows carry `documento <id> | <nome> | <formato> | <N> bytes | sha256 <12>` in `text`.
// The name may contain " | " itself, so the fixed fields are read from the right.
export const lerLinhaUpload = (text: string | null): UploadLog | null => {
  if (!text) return null;
  const partes = text.split(" | ");
  if (partes.length < 5) return null;
  const cabeca = /^documento (\d+)$/.exec(partes[0]);
  const sha = /^sha256 ([0-9a-f]+)$/i.exec(partes[partes.length - 1]);
  const bytes = /^(\d+) bytes$/.exec(partes[partes.length - 2]);
  if (!cabeca || !sha || !bytes) return null;
  const formato = partes[partes.length - 3];
  const nome = partes.slice(1, partes.length - 3).join(" | ");
  return { id: Number(cabeca[1]), nome, formato, bytes: Number(bytes[1]), sha: sha[1] };
};

export const formatarTamanho = (bytes: number): string =>
  bytes < 1024 ? `${bytes} B` : bytes < 1024 * 1024 ? `${(bytes / 1024).toFixed(0)} KB` : `${(bytes / 1048576).toFixed(1)} MB`;

// Short target for the Activity log table: "CV Backend (PDF)"; anything that does not parse falls back.
export const alvoDoUpload = (text: string | null): string | null => {
  const u = lerLinhaUpload(text);
  return u ? `${u.nome} (${u.formato.toUpperCase()})` : null;
};
