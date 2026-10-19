# Camadas de modelo (único lugar com nomes de modelos)

Os papéis e o `CLAUDE.md` falam em CAMADAS. Para trocar de modelo, mude só a coluna "Mapeamento atual".

| Camada | Capacidade | Use para | Mapeamento atual |
|---|---|---|---|
| FORTE | Maior capacidade de raciocínio | Arquiteto, Revisor, tarefa que toca segurança, dados, ferramenta do agente ou arquitetura, tarefa sem precedente no projeto | Opus |
| MÉDIO | Boa qualidade, custo e velocidade equilibrados | QA, lógica nova, vários arquivos, decisão de desenho | Sonnet |
| RÁPIDO | Barato e veloz | Tarefa mecânica e bem delimitada: uma regra, poucos arquivos, renomear, texto, boilerplate | Haiku |

## Regras de escalonamento

- Executor que falha **2 vezes na mesma coisa** sobe uma camada (o orquestrador relança com o mesmo escopo).
- Arquiteto e Revisor ficam sempre em FORTE.
- QA é MÉDIO; sobe para FORTE se os critérios de aceite forem ambíguos.
- Levantamento de requisitos e brainstorm: FORTE se a decisão é ampla, ambígua ou toca segurança, dados ou arquitetura; MÉDIO se o escopo é claro.
- Hierarquia de executores: no máximo 2 níveis (um líder decompõe e delega a folhas). Mais fundo = mais custo e deriva.

Revisar este mapeamento quando saírem modelos novos.
