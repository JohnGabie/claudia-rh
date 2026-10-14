# Pesquisa: lista de sites de vagas (seed do catálogo), por região

> Documento de pesquisa que alimenta `docs/superpowers/specs/2026-10-14-lista-de-sites-design.md` §4.2. **Não é seed, não é código.** Nada daqui entra no app antes de a spec ser aprovada e de o usuário revisar a lista.

**Data:** 2026-10-14
**Estado:** primeira passada concluída para SA, NA, EU, AS e OC. **África e Oriente Médio não foram pesquisados.** Faltam as páginas de empresas de tecnologia e o freelance.

## Como foi feito

- Onze agentes de pesquisa, só leitura, em fontes públicas (busca e leitura de páginas). Nenhum login, nenhuma conta criada, nenhuma candidatura.
- Critério pedido pelo usuário: **os melhores, não necessariamente os mais famosos**. Evidência buscada: vaga direta da empresa ou agregada, vagas fantasmas, golpe, cobrança ao candidato, reputação, transparência de salário.
- **Nada foi reaberto ou conferido por quem consolidou.** Os fatos abaixo são o que cada agente leu e declarou. O que não foi confirmado está escrito "NÃO VERIFICADO".

## Limites desta rodada (valem para tudo abaixo)

1. **A leitura de páginas devolve um resumo, não o texto bruto.** As cláusulas de Termos de Uso marcadas "proíbe" foram citadas literalmente pelos agentes; "sem cláusula encontrada" **não prova permissão**.
2. **Muitos sites bloquearam a leitura (HTTP 403, 404, 503 ou página vazia).** Para eles o domínio vem de busca ou de redirecionamento, e o resto é NÃO VERIFICADO. É a maior parte de Canadá, México, Ásia e de vários portais do Brasil.
3. **Reputação em fóruns (Reddit) e vagas fantasmas** quase não foram verificadas; a reputação vem de Trustpilot, Reclame Aqui (por busca) e de blogs de review, que podem ser de concorrentes ou afiliados.
4. Números de volume ("65 mil vagas") são **afirmações do próprio site**.
5. Os Termos de Uso de **LinkedIn, Indeed, Glassdoor, Catho, Vagas.com, InfoJobs, Seek, Naukri e vários outros não foram lidos**.

## Resumo por região

| Região | Cobertura | Sites que valem olhar primeiro (opinião dos agentes) |
|---|---|---|
| **SA** (Brasil e América Latina) | boa para o Brasil; LATAM só triagem | Vagas.com, Gupy, ProgramaThor, GeekHunter, Get on Board, Bumeran, Elempleo |
| **NA** (EUA, Canadá, México) | EUA boa; Canadá e México só triagem | HiringCafe, USAJOBS, GovernmentJobs, Idealist, Job Bank, OCC |
| **EU** (Dinamarca/Nórdicos, Europa, Reino Unido/Irlanda) | boa | Jobindex, Work in Denmark, The Hub, EU Remote Jobs, Relocate.me, Reed, Hunt UK Visa Sponsors, IrishJobs |
| **AS** (Ásia) | triagem | MyCareersFuture, Japan Dev, Daijob, JobStreet |
| **OC** (Oceania) | triagem | Trade Me Jobs, Seek |
| **AF** (África) | **não pesquisada** | — |
| **GLOBAL** | só os óbvios | LinkedIn, Indeed, We Work Remotely, Himalayas |

## Legenda

- **Papel:** `cand.` = dá para candidatar de ponta a ponta; `desc.` = agregador que só redireciona (`papel = descoberta` na spec).
- **Termos:** `⚠ proíbe` = cláusula contra robôs ou scraping, citada literalmente por um agente. `—` = NÃO VERIFICADO. `sem cláusula` = não achada, o que não prova permissão.
- **Veredito:** INCLUIR, TALVEZ, DESCARTAR, como reportado pelos agentes, com a confiança entre parênteses (alta, média, baixa).

---

## SA: Brasil

| Site | chave | Entrada | Tipo | Papel | Veredito | Termos / notas |
|---|---|---|---|---|---|---|
| Vagas.com | `vagas-com` | https://www.vagas.com.br/login-candidatos | geral | cand. | INCLUIR (alta) | — ; grátis, vaga direta, RA 8,7 (busca) |
| Gupy (portal) | `gupy` | https://login.gupy.io/candidates/signin (reportado por um agente; o outro não viu o link) | empresa/ATS | cand. | INCLUIR (média) | — ; vagas em `*.gupy.io` por empresa; queixas de vagas "tabela" e falta de retorno |
| InfoJobs | `infojobs` | https://www.infojobs.com.br | geral | cand. | INCLUIR (média) | — ; planos de visibilidade pagos, queixas de cobrança |
| ProgramaThor | `programathor` | https://programathor.com.br/users/sign_in | tecnologia | cand. | INCLUIR (média) | — ; mostra empresa e faixa salarial; datas das vagas NÃO VERIFICADO |
| GeekHunter | `geekhunter` | https://www.geekhunter.com/pt/vagas (URL de login NÃO VERIFICADA) | tecnologia | cand. | INCLUIR (média) | — ; matching, a empresa contata |
| Trabalha Brasil | `trabalha-brasil` | https://www.trabalhabrasil.com.br | geral | cand. | TALVEZ (média) | — ; foco operacional; queixas de upsell e acesso (RA 7,1) |
| Empregos.com.br | `empregos-com` | https://www.empregos.com.br/candidato/login.aspx | geral | cand. | TALVEZ (média) | — ; visibilidade do perfil por assinatura |
| BNE | `bne` | https://www.bne.com.br | geral | cand. | TALVEZ (média) | — ; plano VIP opcional |
| Catho | `catho` | https://www.catho.com.br (bloqueou leitura) | geral | cand. | TALVEZ (média) | — ; freemium; ~25% das queixas são cobrança indevida |
| Solides Vagas | `solides` | https://vagas.solides.com.br (um agente teve 404 em `/vagas`, o outro leu a home) | geral | cand. | TALVEZ (baixa) | — |
| Emprega Brasil / SINE | `emprega-brasil` | https://empregabrasil.mte.gov.br (só por busca) | geral (oficial) | cand. | TALVEZ (média) | — ; login gov.br com 2FA; foco operacional |
| Revelo | `revelo` | https://app.careers.revelo.com | tecnologia | cand. | TALVEZ (média) | — ; virou vagas remotas para empresas dos EUA; exige testes |
| Coodesh | `coodesh` | https://coodesh.com/auth/signin | tecnologia | cand. | TALVEZ (baixa) | — ; funil pesado de testes |
| Remotar | `remotar` | https://remotar.com.br (login não localizado) | remoto | desc. | TALVEZ (baixa) | — ; vagas podem ser antigas |
| Trampos.co | `trampos` | https://trampos.co/c/login | geral (criativo) | cand. | TALVEZ (média) | — ; para tecnologia é complementar |
| Nube | `nube` | https://www.nube.com.br | nicho (estágio) | cand. | TALVEZ (baixa) | — |
| CIEE | `ciee` | https://portal.ciee.org.br | nicho (estágio) | cand. | INCLUIR como nicho (média) | — ; ONG; irrelevante para perfil sênior |

**Descartados:** Curriculum.com.br (leitura bloqueada, nada confirmado), Kenoby (absorvida pela Gupy), Trovit (classificados), Pandapé (não confirmado), PrograMaria (sem quadro de vagas).

## SA: América Latina fora do Brasil

| Site | chave | Entrada | Tipo | Papel | Veredito | Termos / notas |
|---|---|---|---|---|---|---|
| Bumeran (grupo: ZonaJobs, Laborum, Multitrabajos, Konzerta, Aptitus) | `bumeran` | https://www.bumeran.com.ar (home veio vazia; domínio por busca) | geral, vários países | cand. | INCLUIR (média) | sem cláusula (só termos de empresas lidos); Aptitus redireciona para `bumeran.com.pe`; contas podem não ser compartilhadas entre marcas |
| Elempleo | `elempleo` | https://www.elempleo.com/co/iniciar-sesion | regional (CO) | cand. | INCLUIR (média) | sem cláusula específica; planos pagos opcionais ao candidato (35 mil a 250 mil COP) |
| Get on Board | `getonboard` | https://www.getonbrd.com | tecnologia | cand. | INCLUIR (média) | — ; foco Chile, México, Colômbia, Peru; salário transparente; muitas vagas em USD |
| Computrabajo | `computrabajo` | https://www.computrabajo.com (subdomínio por país) | geral, ~19 países | cand. | TALVEZ (média) | **⚠ proíbe** ("el acceso o lectura mediante robots o programas automáticos no está permitido"); 590 mil ofertas declaradas |
| Torre | `torre` | https://torre.ai | remoto/tecnologia | cand. | TALVEZ (média) | **⚠ proíbe** automação do processo de candidatura "outside of Torre's official vendors" |
| Magneto365 | `magneto` | https://www.magneto365.com/co | regional (CO) | cand. | TALVEZ (baixa) | — |
| Trabajando.cl | `trabajando` | https://www.trabajando.cl | regional (CL) | cand. | TALVEZ (baixa) | — |
| Chiletrabajos | `chiletrabajos` | https://www.chiletrabajos.cl | regional (CL) | cand. | TALVEZ (baixa) | — |
| Tecoloco | `tecoloco` | https://www.tecoloco.com | regional (América Central) | cand. | TALVEZ (baixa) | — |
| Buscojobs | `buscojobs` | https://www.buscojobs.com.ar | geral | cand. | TALVEZ (baixa) | — ; possível agregação |
| WeRemoto | `weremoto` | https://www.weremoto.com | remoto | desc. | TALVEZ (baixa) | — ; vagas remotas "para LATAM" são alvo de golpe (opinião) |
| Hireline | `hireline` | https://www.hireline.io | tecnologia | cand. | TALVEZ (baixa) | — ; México/LATAM remoto |

**Descartado:** Workana (marketplace de propostas por projeto, vai para a fase freelance). **Não abertos (403):** Gallito Luis (UY), Portal Empleo AR, Trabajo BA.

## NA: Estados Unidos

| Site | chave | Entrada | Tipo | Papel | Veredito | Termos / notas |
|---|---|---|---|---|---|---|
| HiringCafe | `hiringcafe` | https://hiringcafe.com (home 403; domínio por redirecionamento) | geral (agregador de ATS) | desc. | INCLUIR (média) | — ; indexa direto das páginas de carreira, link vai ao ATS da empresa; **apareceu nos dois agentes** |
| USAJOBS | `usajobs` | https://www.usajobs.gov | nicho (governo federal) | cand. | INCLUIR (média) | — ; muitas vagas exigem cidadania; fluxo termina em sistema da agência |
| GovernmentJobs.com (NEOGOV) | `governmentjobs` | https://www.governmentjobs.com | nicho (governo estadual/local) | cand. | INCLUIR (média) | — ; "one application" reutilizável; formulário por órgão |
| Idealist | `idealist` | https://www.idealist.org | nicho (impacto social) | cand. e desc. | INCLUIR (média) | **⚠ proíbe** (§3.d(j): bots, scrapers, crawlers, data mining); fundiu com VolunteerMatch em 2025 |
| Dice | `dice` | https://www.dice.com | tecnologia | desc. | TALVEZ (média) | **⚠ proíbe** (§8.2(j) e (aa); §8.2(k) proíbe treinar IA) |
| Built In | `builtin` | https://builtin.com | tecnologia | desc. | TALVEZ (média) | **⚠ proíbe** (§3(f)); salário transparente |
| Levels.fyi Jobs | `levelsfyi` | https://www.levels.fyi/jobs | tecnologia | desc. | TALVEZ (média) | — (403); botão "Apply" levou ao LinkedIn |
| TrueUp | `trueup` | https://www.trueup.io | tecnologia | desc. | TALVEZ (média) | **⚠ proíbe**, inclusive "artificial intelligence agent" |
| Hacker News Jobs / Who is hiring | `hnjobs` | https://news.ycombinator.com/jobs | tecnologia | desc. | TALVEZ (média) | **⚠ proíbe** ("robots, scraping"); fio mensal não verificado |
| Underdog.io | `underdog` | https://www.underdog.io | tecnologia | cand. (perfil único) | TALVEZ (média) | **⚠ proíbe** (§1.2(e)) |
| Handshake | `handshake` | https://app.joinhandshake.com/access | nicho (estudantes) | cand. | TALVEZ (baixa) | — (404); mistura vagas e tarefas de rotulagem de IA |
| Snagajob | `snagajob` | https://www.snagajob.com | nicho (horista) | cand. | TALVEZ (média) | **⚠ proíbe** (§2.3 e §6); golpes por SMS |
| The Muse | `themuse` | https://www.themuse.com/user/login | geral | cand. e desc. | TALVEZ (média) | **⚠ proíbe** (§10, inclui treino de IA); volume caiu desde 2022 |
| ZipRecruiter | `ziprecruiter` | https://www.ziprecruiter.com (403) | geral | cand. e desc. | TALVEZ (média) | — ; golpes por e-mail e SMS documentados (NBC, CSO) |
| Monster | `monster` | https://www.monster.com (403) | geral | — | TALVEZ (baixa) | — ; falência 2025, vendida à BOLD Holdings |
| CareerBuilder | `careerbuilder` | https://www.careerbuilder.com (403) | geral | — | TALVEZ (baixa) | — ; mesma história da Monster; tratar como entrada única até confirmar |
| Adzuna | `adzuna` | https://www.adzuna.com | geral (agregador) | desc. | TALVEZ (baixa) | — |
| Climatebase | `climatebase` | https://climatebase.org (403) | nicho (clima) | desc. | TALVEZ (baixa) | — ; possível funil para programa pago (inferência) |
| eFinancialCareers | `efinancialcareers` | https://www.efinancialcareers.com | nicho (financeiro) | desc. | TALVEZ (média) | — (404) |
| WayUp | `wayup` | https://www.wayup.com | nicho (estudantes) | cand. | TALVEZ (média) | — (404) |
| Health eCareers | `healthecareers` | — | nicho (saúde) | — | TALVEZ (baixa) | NÃO VERIFICADO (bloqueado) |

**Descartados:** SimplyHired (é a Indeed desde 2016), Lensa, Talent.com (Trustpilot 1,4; spam e vagas velhas), The Ladders (cobrança difícil de cancelar), LinkUp (não é job board), Simplify e Jobright (concorrentes diretos do app; cláusulas mais hostis a agentes de IA e extensões), Work on Climate, Hired (absorvido pela LHH/Adecco), Triplebyte (encerrado em 2023), Stack Overflow Jobs (descontinuado em 2022), GitHub Jobs (NÃO VERIFICADO).
**Descobertos, não pesquisados a fundo:** CareerOneStop, JobGet, Getwork, Jobright, Craigslist, Dribbble Jobs, Authentic Jobs.

**ATS (Greenhouse, Lever, Ashby, Workday):** o agente recomenda **não tratar como "um site só"**, porque cada empresa tem portal, domínio e formulário próprios; o Workday exige conta por empresa. Fica para a fase de empresas de tecnologia.

## NA: Canadá

| Site | chave | Entrada | Tipo | Papel | Veredito | Termos / notas |
|---|---|---|---|---|---|---|
| Job Bank (governo) | `jobbank` | https://www.jobbank.gc.ca/login | geral | cand. (parte) | INCLUIR com ressalva (média) | **⚠ proíbe** ("script, robot, spider, Web crawler, screen scraper, automated query program, artificial intelligence or other automated device"); inclui vagas com LMIA |
| GC Jobs (serviço público federal) | `gcjobs` | https://emploisfp-psjobs.cfp-psc.gc.ca (URL não aberta) | nicho (setor público) | cand. | TALVEZ (média) | — ; burocrático, mal adaptado à automação |
| Talent.com CA | `talentca` | https://ca.talent.com | geral (agregador) | desc. | TALVEZ (média) | **⚠ proíbe** (robôs, offline readers) |
| Jobillico | `jobillico` | https://www.jobillico.com (403) | regional (QC) | cand. | TALVEZ (baixa) | — |
| Jobboom | `jobboom` | https://www.jobboom.com (403) | regional (QC) | — | TALVEZ (baixa) | — |
| Eluta.ca | `eluta` | https://www.eluta.ca (403) | geral | desc. | TALVEZ (baixa) | — |
| Work In Tech (Communitech) | `worktech` | https://www1.communitech.ca/jobs (redirecionamento não aberto) | tecnologia | desc. | TALVEZ (baixa) | — |

**Descartado:** Workopolis (aparenta ser portal white-label da Indeed; só pelo título de um resultado de busca). **Citados sem verificação:** Magnet, CharityVillage, MaRS, Jobs.ca.

## NA: México

| Site | chave | Entrada | Tipo | Papel | Veredito | Termos / notas |
|---|---|---|---|---|---|---|
| OCC Mundial | `occ` | https://www.occ.com.mx (403; domínio por busca) | geral | cand. | INCLUIR (média) | — (404); publicou alerta de que cobrar do candidato é vaga falsa |
| Computrabajo MX | (ver `computrabajo` em SA) | https://mx.computrabajo.com | geral | cand. | veredito do agente do México: INCLUIR; do agente LATAM: TALVEZ | **⚠ proíbe**; ficou TALVEZ pela cláusula citada |
| Bumeran MX | (ver `bumeran` em SA) | https://www.bumeran.com.mx | geral | cand. | TALVEZ (baixa) | — |
| Portal del Empleo (STPS/SNE) | `empleogob` | https://www.empleo.gob.mx | geral (oficial) | cand. (parte) | TALVEZ (baixa) | — ; pouco adequado para vaga profissional |
| Hireline | (ver SA) | https://www.hireline.io | tecnologia | cand. | TALVEZ (baixa) | — |

Golpes de vaga falsa são frequentes no México em 2026 (alertas oficiais), sem evidência específica contra a OCC.

## EU: Dinamarca e Nórdicos

| Site | chave | Entrada | Tipo | Idioma | Veredito | Termos / notas |
|---|---|---|---|---|---|---|
| Jobindex | `jobindex` | https://www.jobindex.dk/bruger/log-ind | geral (DK) | da/en | INCLUIR (alta) | sem cláusula na página lida; termos comerciais (`/handelsbetingelser`) não lidos; absorveu o Ofir em 2025 |
| Work in Denmark | `workindenmark` | https://www.workindenmark.dk | geral (oficial, DK) | en | INCLUIR (média) | — ; só política de privacidade |
| The Hub | `thehub` | https://thehub.io | tecnologia (startups, NORDIC) | en | INCLUIR (média) | sem cláusula (`/terms`) |
| Jobnet | `jobnet` | https://jobnet.dk | geral (oficial, DK) | da | TALVEZ (média) | — ; **conta exige MitID** (identidade dinamarquesa) |
| EURES | `eures` | https://eures.europa.eu/index_en | geral (oficial UE) | ambos | TALVEZ (média) | sem cláusula; muita repetição de vagas |
| StepStone DK | `stepstonedk` | https://www.stepstone.dk | geral (liderança, DK) | da/en | TALVEZ (média) | sem cláusula; **operado pelo Jobindex A/S** |
| Computerworld it-jobbank | `itjobbank` | https://www.it-jobbank.dk | tecnologia (DK) | da/en | TALVEZ (média) | — ; mesmo grupo do Jobindex |
| Jobs in Copenhagen | `jobsincopenhagen` | https://jobsincopenhagen.com | geral (expat, DK) | en | TALVEZ (baixa) | — ; rede de boards em massa |
| Vacancies.dk | `vacanciesdk` | https://vacancies.dk | geral (expat, DK) | en | TALVEZ (baixa) | sem cláusula; site de uma pessoa |
| Platsbanken | `platsbanken` | https://arbetsformedlingen.se/platsbanken/ | geral (oficial, SE) | sv/en | TALVEZ (média) | — ; existe API aberta oficial |
| FINN.no Jobb | `finnjobb` | https://www.finn.no/job/ | geral (NO) | no | TALVEZ (média) | — |
| Jobbnorge | `jobbnorge` | https://www.jobbnorge.no/en | geral (público/pesquisa, NO) | no/en | TALVEZ (baixa) | — |
| Duunitori | `duunitori` | https://duunitori.fi | geral (FI) | fi/en | TALVEZ (média) | — |

**Descartados:** Jobs in English (cobra do candidato **e** proíbe scraping, §4), Ofir (fechou; domínio responde 410), Graduateland (redireciona para JobTeaser), Jobbank.dk (403). **Não abertos:** Jobbsafari, Jobunivers/Karriere.dk, Jobylon, Nav.no, Tyomarkkinatori.

## EU: Europa e remoto europeu

| Site | chave | Entrada | Tipo | Papel | Veredito | Termos / notas |
|---|---|---|---|---|---|---|
| EU Remote Jobs | `euremotejobs` | https://euremotejobs.com | remoto (EU) | desc. (link ao ATS; sem login) | INCLUIR (média) | **⚠ proíbe** (robôs, scrapers, "bulk downloading") |
| Relocate.me | `relocate` | https://relocate.me | tecnologia (EU/GLOBAL) | cand. | INCLUIR (média) | sem cláusula (termos dentro de `/privacy`); volume pequeno |
| Landing.jobs | `landingjobs` | https://landing.jobs | tecnologia (PT/EU) | cand. | INCLUIR (média) | **⚠ proíbe** (§8.2(vii), inclui "monitor") |
| Arbeitnow | `arbeitnow` | https://www.arbeitnow.com | regional (DE) | desc. (link ao ATS; sem login) | INCLUIR (média) | sem cláusula (só engenharia reversa); só Alemanha |
| Welcome to the Jungle (inclui Otta) | `wttj` | https://www.welcometothejungle.com/en | geral (FR/UK/US) | cand. | TALVEZ (média) | **⚠ proíbe** (inclui "browser plug-ins"); fraco na Dinamarca |
| Berlin Startup Jobs | `berlinstartupjobs` | https://berlinstartupjobs.com | regional (DE) | — | TALVEZ (baixa) | proíbe nos termos de cliente; sem datas nas vagas |
| Next Level Jobs EU | `nextleveljobs` | https://nextleveljobs.eu | tecnologia (EU) | desc. | TALVEZ (baixa) | — ; candidato paga (€39 a €109) |

**Descartados:** Honeypot (só por convite, site não abriu), SwissDevJobs (só Suíça, 403). **Não pesquisados:** Xing, StepStone internacional (relevantes se DACH importar).

## EU: Reino Unido e Irlanda

| Site | chave | Entrada | Tipo | Papel | Veredito | Termos / notas |
|---|---|---|---|---|---|---|
| Reed | `reed` | https://www.reed.co.uk | geral | cand. | INCLUIR (média) | sem cláusula de robôs (só proíbe interferir no site); **filtro de visa sponsorship lido** (1.490 vagas); muitas vagas de agência |
| Find a job (DWP/GOV.UK) | `findajob_gov` | https://www.gov.uk/find-a-job | geral (oficial) | cand. | INCLUIR (média) | — (503); grátis, estável; sem filtro de visa confirmado |
| CV-Library | `cvlibrary` | https://www.cv-library.co.uk | geral | cand. | INCLUIR (média) | — (403); filtro de visa só por fonte secundária |
| Hunt UK Visa Sponsors | `huntukvisas` | https://huntukvisasponsors.com | nicho (visa) | desc. (link direto à empresa) | INCLUIR (média) | **⚠ proíbe** ("scrape, crawl or bulk-download... outside our API"); cruza com o registro oficial de sponsors; paywall nas vagas recentes |
| IrishJobs.ie | `irishjobs` | https://www.irishjobs.ie (403) | regional (IE) | cand. | INCLUIR (média) | — (403); sem filtro de visa confiável |
| NHS Jobs | `nhsjobs` | https://www.jobs.nhs.uk (403) | nicho (saúde) | cand. | TALVEZ (média) | — ; só para perfil de saúde |
| Totaljobs (+ CWJobs, Jobsite) | `totaljobs` | https://www.totaljobs.com (403) | geral | cand. | TALVEZ (média) | — ; CWJobs é o portal de tecnologia da rede |
| UK Visa Jobs | `ukvisajobs` | https://ukvisajobs.com | nicho (visa) | desc. | TALVEZ (baixa) | **⚠ proíbe** ("spider, crawl, or scrape"); assinatura com renovação automática |
| Adzuna UK | `adzuna_uk` | https://www.adzuna.co.uk (403) | geral (agregador) | desc. | TALVEZ (média) | — (só os termos da API foram lidos) |
| Guardian Jobs | `guardianjobs` | https://jobs.guardian.co.uk | nicho | cand. | TALVEZ (baixa) | — |
| Jobs.ie | `jobsie` | https://www.jobs.ie (timeout) | regional (IE) | cand. | TALVEZ (baixa) | — ; sobreposição com IrishJobs |
| RecruitIreland | `recruitireland` | https://www.recruitireland.com | regional (IE) | cand. | TALVEZ (baixa) | sem cláusula; atividade em 2026 não provada |
| Publicjobs.ie | `publicjobs` | https://www.publicjobs.ie/en/ | nicho (setor público, IE) | cand. | TALVEZ (média) | — ; elegibilidade por nacionalidade não confirmada |

**Descartados:** ukworkx.com (cobrou £267 sem entrega, Trustpilot por busca), hiremyvisa.com (sem evidência), Jobsite como entrada separada (parte da rede Totaljobs). **Sem veredito:** Prospects, Technojobs, Cord, Work in Startups, Jobbio.

## AS: Ásia

| Site | chave | Entrada | Tipo | Papel | Veredito | Termos / notas |
|---|---|---|---|---|---|---|
| MyCareersFuture | `mycareersfuture` | https://www.mycareersfuture.gov.sg | regional (governo, SG) | cand. | INCLUIR (média) | — ; empresas são obrigadas a anunciar lá por 14 dias antes de pedir EP/S Pass; pode exigir Singpass |
| JobStreet (JobsDB em HK/TH) | `jobstreet` | https://sg.jobstreet.com (403) | regional (SG/MY/PH/ID) | cand. | INCLUIR (média) | — ; mesma plataforma do Seek |
| Japan Dev | `japandev` | https://japan-dev.com | tecnologia (JP) | cand. ou redireciona | INCLUIR (média) | — (404); 279 vagas, salário e tag "Apply from Abroad" |
| Daijob | `daijob` | https://www.daijob.com/en/ | nicho (bilíngue, JP) | cand. | INCLUIR (média) | sem cláusula (só "violation of authorization and security system"); foco sênior |
| GaijinPot Jobs | `gaijinpot` | https://jobs.gaijinpot.com (403) | nicho (estrangeiros, JP) | cand. | TALVEZ (média) | — ; muito ensino de inglês |
| Cutshort | `cutshort` | https://cutshort.io | tecnologia (IN) | cand. (reverso) | TALVEZ (média) | **⚠ proíbe** (§7.2.9: "automated process or service") |
| Hirist | `hirist` | https://www.hirist.tech | tecnologia (IN) | cand. | TALVEZ (média) | **⚠ proíbe** ("exclude Robots... not to circumvent") |
| Naukri | `naukri` | https://www.naukri.com | geral (IN) | cand. | TALVEZ (média) | paráfrase: age contra "crawling any data"; serviços pagos de destaque |
| Kalibrr | `kalibrr` | http://www.kalibrr.com/home | regional (PH) | cand. | TALVEZ (média) | **⚠ proíbe** ("automated extraction, scraping") |
| Instahyre | `instahyre` | https://www.instahyre.com (403) | tecnologia (IN) | — | TALVEZ (baixa) | — |
| eFinancialCareers SG | `efinancialcareers` | https://www.efinancialcareers.sg | nicho (finanças, SG) | cand. | TALVEZ (baixa) | — (404) |

**Coreia:** Wanted e KOWORK são candidatos (TALVEZ, baixa, não abertos). **KLiK (JobKorea) foi encerrado em 2026-06-08.** Saramin e JobKorea só em coreano. **China continental:** não pesquisada. **Hong Kong:** CTgoodjobs e Recruit.com.hk citados, não verificados.
**Descartado:** Wantedly (japonês, candidatura informal). **Não abertos:** CareerCross, Foundit, Glints, NodeFlair, TokyoDev, Jobs in Japan.

## OC: Oceania

| Site | chave | Entrada | Tipo | Papel | Veredito | Termos / notas |
|---|---|---|---|---|---|---|
| Trade Me Jobs | `trademe` | https://www.trademe.co.nz/a/jobs | regional (NZ) | cand. | INCLUIR (média) | — (404); 10.468 vagas lidas na página |
| Seek | `seek` | https://www.seek.com.au (403) | geral (AU/NZ) | cand. | TALVEZ (média) | — ; Trustpilot 1,4/5 ("90%+ of the jobs didn't even get viewed"); anti-bot provável; dono do Jora e do JobStreet |
| Jora | `jora` | https://au.jora.com (403) | geral (AU) | desc. | TALVEZ (baixa) | — ; golpes por SMS usando a marca |
| Ethical Jobs | `ethicaljobs` | https://www.ethicaljobs.com.au | nicho (terceiro setor, AU) | cand. | TALVEZ (média) | sem cláusula; texto completo não lido |

## GLOBAL: os óbvios e o remoto sem região

| Site | chave | Entrada | Tipo | Papel | Veredito | Termos / notas |
|---|---|---|---|---|---|---|
| LinkedIn | `linkedin` | https://www.linkedin.com/jobs/ | geral | cand. | INCLUIR (alta) | **termos NÃO LIDOS**; golpes de recrutador falso documentados (Febraban, 2026) |
| Indeed | `indeed` | https://br.indeed.com (403) e domínios por país | geral (agregador) | cand. e desc. | INCLUIR (média) | **termos NÃO LIDOS**; parte redireciona; bloqueio de bot provável |
| We Work Remotely | `weworkremotely` | https://weworkremotely.com (403) | remoto | desc. | INCLUIR (média) | — ; termos de API proíbem scraping (via busca); muitas vagas só para EUA |
| Himalayas | `himalayas` | https://himalayas.app | remoto | cand. | INCLUIR (média) | **⚠ proíbe** ("robots, screen scraping") |
| Glassdoor | `glassdoor` | https://www.glassdoor.com.br (403) | geral/avaliações | cand. | TALVEZ (média) | — ; segundo a Wikipedia, fundida à Indeed em 2026-07-01: redundante como fonte de vagas |
| Jooble | `jooble` | https://br.jooble.org | geral (agregador) | desc. | TALVEZ (média) | — ; só redireciona |
| Remotive | `remotive` | https://remotive.com | remoto | cand. | TALVEZ (média) | **⚠ proíbe**; paywall para ver quase tudo |
| Working Nomads | `workingnomads` | https://www.workingnomads.com/jobs | remoto | — | TALVEZ (média) | sem cláusula; alerta de sites falsos que imitam o domínio |
| Remote OK | `remoteok` | https://remoteok.com | remoto | — | TALVEZ (baixa) | sem cláusula (só atribuição de API); filtros pagos |
| Jobicy | `jobicy` | https://jobicy.com | remoto | — | TALVEZ (baixa) | — |
| Wellfound | `wellfound` | https://wellfound.com/login | tecnologia (foco EUA) | cand. | TALVEZ (média) | **⚠ proíbe, condicional à carga** (a mais tolerante); vagas somem e voltam (relatos) |
| YC Work at a Startup | `ycwaas` | https://www.ycombinator.com/jobs | tecnologia (foco EUA) | cand. | TALVEZ (média) | **⚠ proíbe** |
| Arc.dev | `arc` | https://arc.dev/login | remoto/freelance | rede vetada | TALVEZ (média) | **⚠ proíbe** (§7.7(k)) |
| Jobgether | `jobgether` | https://jobgether.com | remoto (agregador) | cand. | TALVEZ (baixa) | **⚠ proíbe** (§4.5, inclui treinar modelos) |

**Descartados:** FlexJobs (pago ao candidato; termos proíbem ferramentas que preenchem candidaturas), Remote.co, Remote Rocketship (paywall).

## AF: África e Oriente Médio

**Não pesquisados.** Lacuna declarada, e não esquecimento.

---

## Achados que afetam a spec

1. **Os termos proíbem automação, quase sempre.** Dos sites com termos lidos, a grande maioria proíbe por escrito robôs, scrapers e "acesso automatizado", inclusive **portais oficiais** (Job Bank do Canadá). Só o Wellfound limita a proibição à carga que um humano não produziria. **Sem cláusula encontrada não é permissão.** Os termos de LinkedIn, Indeed e Glassdoor, os de maior risco conhecido, continuam não lidos.
2. **O risco prático é bloqueio de conta, e não ação judicial** (opinião de um agente), e pesa pouco quando a candidatura vai para o ATS da empresa. Isso é uma **decisão de risco do usuário**; a spec precisa registrá-lo e, se o usuário quiser, ganhar um campo por site com o resultado da leitura dos termos (proposta, não decidida).
3. **A rigidez do MCP e os agregadores se chocam.** Indeed, Jooble, Remotar, HiringCafe e outros redirecionam para sites de empresa fora da lista. **Decidido pelo usuário:** entram como `descoberta` (campo `papel`), e a contagem de domínios recusados mede o custo.
4. **Taxonomia:** estágio, PCD e afins pedem um tipo `nicho`. **Decidido pelo usuário:** sétimo tipo `nicho` (spec P9).
5. **Região:** `pais` misturava países e blocos (NORDIC, EU, LATAM). **Decidido pelo usuário:** campo `regiao` e filtro (spec P11). Vários sites cobrem muitos países (Computrabajo, Bumeran, Torre, Buscojobs) e ficam com `pais` vazio.
6. **Grupos e marcas absorvidas:** Jobindex = StepStone DK = it-jobbank; Seek = Jora = JobStreet/JobsDB; Indeed = SimplyHired = Workopolis (provável) = Glassdoor (2026); Monster = CareerBuilder (BOLD); Bumeran = ZonaJobs = Laborum = Multitrabajos = Aptitus. O catálogo deve tratar o grupo com cuidado: **a mesma conta pode não valer entre marcas** (não verificado).
7. **Exigências de identidade:** Jobnet (MitID), MyCareersFuture (Singpass provável), Emprega Brasil (gov.br com 2FA), USAJOBS (cidadania em muitas vagas). A IA provavelmente **não consegue conta** nesses.
8. **Conflitos entre agentes:** Computrabajo MX (INCLUIR de um, TALVEZ do outro, que citou a cláusula); a URL de login da Gupy; `vagas.solides.com.br`; Idealist (um leu a cláusula, o outro teve 404). Prevalece quem citou a fonte.

## Lacunas para a próxima passada de verificação

1. **Ler os Termos de Uso, com navegador real, dos sites que bloquearam a leitura**, começando por LinkedIn, Indeed, Glassdoor, Catho, Seek, Naukri, OCC, Computrabajo e Totaljobs.
2. Abrir as URLs de login marcadas NÃO VERIFICADO (GeekHunter, Get on Board, Remotar, Nube, Solides, Gupy, Himalayas).
3. Conferir datas de vagas e reputação (Reclame Aqui, Reddit) dos INCLUIR.
4. Confirmar o portal atual do SINE, se `glassdoor.com.br` ainda é independente e o estado de Monster e CareerBuilder.
5. Pesquisar **África e Oriente Médio**, China e Hong Kong, Coreia, e PCD e mulheres em tecnologia com quadro real.

## Próximas fases

- **Empresas de tecnologia:** páginas de carreiras e ATS usados por elas.
- **Freelance** (Workana, Upwork e afins; Arc.dev e Workana já apareceram).
