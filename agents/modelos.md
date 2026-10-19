# Camadas de modelo (único lugar com nomes de modelos)

Os papéis e o `CLAUDE.md` falam em CAMADAS. Para trocar de modelo, mude só a coluna "Mapeamento atual".

| Camada | Capacidade | Use para | Mapeamento atual |
|---|---|---|---|
| FORTE | Maior capacidade de raciocínio | Arquiteto, Revisor, QA (recomendado), tarefa que toca segurança, dados, ferramenta do agente ou arquitetura, tarefa sem precedente no projeto | Opus |
| MÉDIO | Boa qualidade, custo e velocidade equilibrados | Lógica nova, vários arquivos, decisão de desenho | Sonnet |
| RÁPIDO | Barato e veloz | Tarefa mecânica e bem delimitada: uma regra, poucos arquivos, renomear, texto, boilerplate | Haiku |

## Regras de escalonamento

- Executor que falha **2 vezes na mesma coisa** sobe uma camada (o orquestrador relança com o mesmo escopo).
- Arquiteto e Revisor ficam sempre em FORTE.
- QA: FORTE recomendado (assim era antes); MÉDIO só com critérios claros e mudança pequena.
- Levantamento de requisitos e brainstorm: é trabalho do Arquiteto, logo FORTE.
- Hierarquia de executores: no máximo 2 níveis (um líder decompõe e delega a folhas). Mais fundo = mais custo e deriva.

Revisar este mapeamento quando saírem modelos novos.
