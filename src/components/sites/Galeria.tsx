import React from "react";
import { Cartao } from "./Cartao";
import type { ListaProps } from "./tipos";

export const Galeria: React.FC<ListaProps> = ({ sites, erros, ...resto }) => (
  <div className="st-grade">
    {sites.map((s) => <Cartao key={s.chave} site={s} erro={erros[s.chave]} {...resto} />)}
  </div>
);
