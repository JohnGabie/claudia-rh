# Pesquisa: lista de sites de vagas (seed do catálogo), por região

> Documento de pesquisa que alimenta `docs/superpowers/specs/2026-10-14-lista-de-sites-design.md` §4.2. **Não é seed, não é código.** Nada daqui entra no app antes de a spec ser aprovada e de o usuário revisar a lista.

**Data:** 2026-10-14
**Estado:** primeira passada concluída para SA, NA, EU, AS e OC, para empresas de tecnologia (globais, regionais, corporativas "normais" e "legais") e para freelance. **África e Oriente Médio não foram pesquisados.** Falta a passada de verificação dos termos e das URLs de login.

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

## EMPRESAS: páginas de carreiras de tecnologia (`tipo = empresa`)

Aqui a unidade é a **empresa**, e não o ATS: cada uma tem a página de carreiras e, quase sempre, **outro domínio** para onde a candidatura vai. **Nenhuma cláusula de robôs foi confirmada em nenhuma delas**; os termos de Greenhouse, Lever, Workday, Teamtailor e Gupy deram 404. Isso é NÃO VERIFICADO, e não permissão.

### Revisão pelo critério único (decisão do usuário, 2026-10-14)

**Pergunta:** a empresa publica as próprias vagas num site próprio? **Se sim, mantém.** Lida como: existe página de carreiras no domínio da empresa (ou subdomínio dela) que lista as vagas ou leva direto a elas, mesmo que o formulário final fique num ATS. **Este critério substitui os vereditos INCLUIR/TALVEZ das tabelas abaixo para empresas;** as tabelas ficam como registro de ATS e domínios.

**SIM, mantém (77):**

| Grupo | Empresas |
|---|---|
| Globais | Anthropic, Google, Microsoft, Amazon, Apple, Netflix, Nvidia, OpenAI, Stripe, Databricks, Cloudflare, Meta, Salesforce, Shopify, Atlassian, Snowflake |
| Corporativas | Accenture, LEGO, Ørsted, SAP, Dell, HP, IBM, Cisco, Nordea, Capgemini, Ericsson, Grundfos, Carlsberg, Kyndryl, Intel, Siemens, Deloitte DK, Arla, Danske Bank |
| Europa e Nórdicos | Pleo, Lunar, Templafy, Spotify, DeepL, Klarna, Netcompany, Vestas, Unity, Zalando, Booking.com, Adyen, Wise, N26, Celonis, Novo Nordisk |
| Brasil | Nubank, Zup, QuintoAndar, Magalu |
| "Legais" | Canonical, GitLab, Supabase, Grafana Labs, Elastic, Linear, Automattic, Mozilla, Sentry, Tailscale, Mistral AI, JetBrains, Doist, Netlify, Prisma, Vercel, Red Hat, SUSE, Cohere, Docker, Fly.io, 37signals |

**Segunda passada (mesmo dia), sobre os 16 pendentes:**

| Resultado | Empresas |
|---|---|
| **SIM, página aberta** (+4 aos 77) | iFood (`carreiras.ifood.com.br`), Stone (`jornada.stone.com.br`, cerca de 357 vagas por área), Trustpilot (`corporate.trustpilot.com/careers`, botão "View open jobs" leva a um ATS não identificado), Oracle (`careers.oracle.com`, abriu sem erro, resumo vago) |
| **SIM, URL dada pelo usuário** (+4; não reabertas por agente) | Mercado Libre (`https://careers-meli.mercadolibre.com/pt/positions?country=<país>`; o parâmetro `country` é obrigatório), Revolut (`https://www.revolut.com/careers/`), Personio (`https://www.personio.com/about-personio/careers/`), Nokia (`https://jobs.nokia.com/en/sites/CX_1`, subdomínio `jobs.` e não `nokia.com/careers`, que era o que o agente tentava) |
| **Ainda NÃO VERIFICADO (8), fora do seed até confirmar** | VTEX (`careers.vtex.com` recusou conexão; `vtex.com/br-pt/carreiras/` 404), TOTVS (402 e DNS), Maersk (erro de leitura), Hugging Face (`huggingface.co/jobs` abriu a home; só o ATS `apply.workable.com/huggingface` lista vagas), Creditas (404 e DNS), Globo (domínios bloqueados pela ferramenta), Siteimprove (404), Mollie (404 e DNS) |

**Placar:** 81 SIM lidos + 4 SIM por conferência do usuário = 85 mantidos; 8 pendentes; nenhum NÃO firme.

**Terceira passada, com navegador (Playwright, janela própria, só leitura de páginas públicas):**

| Empresa | Resultado | Evidência |
|---|---|---|
| TOTVS | **SIM** | `https://www.totvs.com/trabalhe-conosco/` abriu; o botão "Confira as vagas na TOTVS" leva a `atracaodetalentos.totvs.app/vempratotvs/extended` (domínio da própria TOTVS) |
| Maersk | **SIM** | `https://www.maersk.com/careers/vacancies` abriu; título "Vacancies \| Careers at Maersk" |
| VTEX | **só link direto ao ATS** | `vtex.com/pt-br/carreiras/` deu 404; o único link de carreiras do site é `https://job-boards.greenhouse.io/vtex` |
| Hugging Face | **só link direto ao ATS** | o "Careers" do site aponta direto para `https://apply.workable.com/huggingface/`; `huggingface.co/jobs` não é página de carreiras |
| Creditas | **sem página própria vista** | a home não tem link de carreiras; a busca "Creditas trabalhe conosco" devolveu o portal da Creditas no Gupy, LinkedIn e Glassdoor; inconclusivo |
| Mollie | **SIM** | o site (`mollie.com/pt/`) linka "Carreiras" para `https://jobs.mollie.com/` |
| Siteimprove | **SIM** | o site linka "Careers" para `https://careers.siteimprove.com/` |
| Globo | **SIM (por busca)** | a busca "Globo trabalhe conosco" mostra "Vem pra Globo — Plataforma de Carreiras" e "Trabalhe Conosco - Grupo Globo" em `grupoglobo.com.br`; as páginas em si não foram abertas |

**Pergunta aberta ao usuário:** VTEX e Hugging Face (o site só linka direto para o quadro do ATS, sem página de carreiras) passam no critério "posta as vagas no próprio site"? Grafana e Zup, que têm página própria com botão para o ATS, foram mantidas.

**Por que não zerou:** a ferramenta de busca atingiu o teto da sessão (`web search budget (200 of 200)`). As duas passadas tiveram de usar tentativa direta de domínios, e 404, DNS e 403 não provam ausência de página. Para fechar: elevar `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` e abrir sessão nova, ou o usuário colar as URLs.

**NÃO firme:** nenhuma. Nenhuma empresa ficou provada como "só em portal de terceiro".

**Ressalvas dos SIM:**
- **Só a página leva ao ATS, sem listar vagas:** Grafana Labs e Zup (botão para o quadro do Greenhouse).
- **Leitura parcial:** Intel (o Workday final veio vazio), Grundfos (6 subpáginas não abertas), Celonis (vagas por API; o texto lido mostrava "no jobs at the moment"), Meta e Salesforce (conteúdo mínimo; SIM pelo domínio e pelo redirecionamento).
- **Magalu passa no critério**, embora um agente anterior tenha dito que a candidatura vai para a plataforma 99jobs (terceiro).
- **37signals** passa, mas **não tem vagas hoje**.
- **Idioma:** Lunar e Netcompany pedem dinamarquês em muitas vagas.

### Revisão por reputação como empregador (decisão do usuário, 2026-10-14)

**Critério:** nota geral do Glassdoor **abaixo de 3,0 de 5 sai**. Notas lidas por agentes em snippets do Google e do DuckDuckGo em 2026-10-14, **sem abrir o Glassdoor e sem data de coleta**; o "% recomenda" vem em parte de resumo gerado por IA do Google. Notícias de peso (demissão, assédio) **não foram pesquisadas**.

**Saem (3):** **DeepL**, nota 2,7 (~32% recomendam; alta gestão 2,1), e, por decisão do usuário ("pode remover, sem dó"), **Mozilla** (3,0; 38% recomendam, 22% abaixo da média) e **Docker** (3,0; 45% recomendam, reviews citam caos), que estavam exatamente no limite.

**Ficam, mas com alerta (nota acima de 3,0 e % recomenda baixo):** Klarna 3,1 (35 a 37%), Templafy 3,2 (46%), Pleo 3,2 (51%), Automattic 3,1, Atlassian 3,2, Cloudflare 3,2, Personio 3,2, Shopify 3,3, GitLab 3,3 (queda de 17% em 12 meses), Meta 3,4 (queda de 9%), Canonical 3,4 (57% recomendam; reviews citam gestão fraca e ritmo exigente).

**Nota não confiável (poucas avaliações ou dado ambíguo):** Supabase 4,9 (9), 37signals 5,0 (3), Tailscale 4,2 (11), Doist 4,5 (29), Zup 4,3 (58, conflito com outro número), Anthropic 4,3 (39), Mistral AI (2 a 3, páginas se contradizem), Fly.io (1), Cohere (15 a 40), Prisma (pode ser outra empresa homônima), **Linear (sem dado**; só achou empresas homônimas).

**Melhores notas com amostra razoável:** Nvidia 4,6, Mollie 4,4, Google 4,4, Capgemini 4,3, Nubank 4,3, Revolut 4,2 a 4,3, Siteimprove 4,2, LEGO 4,2, SAP 4,2, iFood 4,2, Booking 4,1, TOTVS 4,1, Globo 4,1, Red Hat 4,1, Grafana Labs 4,1.

**Ficam no seed sem restrição de nota:** todas as demais (nota entre 3,4 e 4,1).

### Globais (`regiao = GLOBAL`)

| Empresa | chave | Carreiras | Domínios envolvidos | ATS | Veredito |
|---|---|---|---|---|---|
| Anthropic | `anthropic` | https://job-boards.greenhouse.io/anthropic | `job-boards.greenhouse.io` (`boards.greenhouse.io` redireciona) | Greenhouse | INCLUIR (alta) |
| Google | `google` | https://www.google.com/about/careers/applications/ | `www.google.com`; `careers.google.com` redireciona | próprio | INCLUIR (média) |
| Microsoft | `microsoft` | https://apply.careers.microsoft.com/careers | `careers.microsoft.com` redireciona para `apply.careers.microsoft.com` | próprio (Eightfold, conhecimento prévio) | INCLUIR (média) |
| Amazon | `amazon` | https://www.amazon.jobs/en/ | `amazon.jobs` | próprio | INCLUIR (média) |
| Apple | `apple` | https://jobs.apple.com/en-us/search | `jobs.apple.com` | próprio | INCLUIR (média) |
| Netflix | `netflix` | https://explore.jobs.netflix.net/careers | `explore.jobs.netflix.net` | Eightfold | INCLUIR (média) |
| Nvidia | `nvidia` | https://jobs.nvidia.com/careers | `jobs.nvidia.com`; `nvidia.wd5.myworkdayjobs.com` (fonte de terceiros) | Workday | INCLUIR (média) |
| OpenAI | `openai` | https://openai.com/careers/ (403) | `openai.com`; `jobs.ashbyhq.com` (terceiros) | Ashby | INCLUIR (média) |
| Stripe | `stripe` | https://stripe.com/jobs | `stripe.com`; Greenhouse | Greenhouse (inferência) | INCLUIR (média) |
| Databricks | `databricks` | https://www.databricks.com/company/careers | `databricks.com`; Greenhouse (terceiros) | Greenhouse | INCLUIR (média) |
| Cloudflare | `cloudflare` | https://www.cloudflare.com/careers/jobs/ | `cloudflare.com`; `job-boards.greenhouse.io` (terceiros) | Greenhouse | INCLUIR (média) |
| Meta | `meta` | https://www.metacareers.com/ | `metacareers.com` | próprio | TALVEZ (baixa) |
| Salesforce | `salesforce` | https://www.salesforce.com/company/careers/jobs/ | `careers.salesforce.com` redireciona | Workday (conhecimento prévio) | TALVEZ (baixa) |
| Shopify | `shopify` | https://www.shopify.com/careers | `shopify.com` | Ashby (fraca) | TALVEZ (baixa) |
| Atlassian | `atlassian` | https://www.atlassian.com/company/careers | `atlassian.com` | próprio (terceiros) | TALVEZ (baixa) |
| Snowflake | `snowflake` | — (`/en/careers/` deu 404) | — | — | NÃO VERIFICADO |

**Citadas só por conhecimento prévio, não verificadas hoje:** Adobe, Oracle, SAP, IBM, Cisco, Intel, Datadog, GitLab, Airbnb, Spotify (verificada abaixo), Uber, Twilio, MongoDB, Elastic, Canonical.

### Regionais

| Empresa | chave | Reg/País | Carreiras | ATS (domínio de candidatura) | Veredito |
|---|---|---|---|---|---|
| Pleo | `pleo` | EU/DK | https://www.pleo.io/en/careers | Ashby (`jobs.ashbyhq.com/pleo`) | INCLUIR (alta); remoto em vários países e híbrido em Copenhagen |
| Lunar | `lunar` | EU/DK | https://jobs.lunar.app/jobs | Teamtailor (`jobs.lunar.app`) | INCLUIR (média); muitas vagas pedem dinamarquês; remoto improvável |
| Templafy | `templafy` | EU/DK | https://templafy.com/careers/ | Teamtailor (`templafy.teamtailor.com`) | INCLUIR (média); híbrido |
| Spotify | `spotify` | EU/SE | https://www.lifeatspotify.com/jobs | Lever (`jobs.lever.co/spotify`); uma fonte diz Greenhouse | INCLUIR (média); nenhuma vaga em Copenhagen na amostra |
| DeepL | `deepl` | EU/DE | https://www.deepl.com/en/careers | Ashby (`jobs.ashbyhq.com/DeepL`) | INCLUIR (média) |
| Nubank | `nubank` | SA/BR | https://nu.com/pt/carreiras | Ashby (`jobs.ashbyhq.com/nubank`); `nubank.com.br/carreiras` redireciona | INCLUIR (média); AR, BR, CA, CO, DE, MX, US, UY |
| VTEX | `vtex` | SA/BR | https://job-boards.greenhouse.io/vtex | Greenhouse | INCLUIR (média); nenhuma vaga marcada como remota |
| Klarna | `klarna` | EU/SE | https://www.klarna.com/careers/ | Deel (`jobs.deel.com/job-boards/klarna`) | TALVEZ (média) |
| Netcompany | `netcompany` | EU/DK | https://www.netcompany.com/careers | não verificado | TALVEZ (média); muitas vagas exigem dinamarquês |
| Vestas | `vestas` | EU/DK | https://careers.vestas.com/search | próprio | TALVEZ (média); 638 vagas |
| Trustpilot | `trustpilot` | EU/DK | https://corporate.trustpilot.com/careers | não verificado | TALVEZ (baixa) |
| Danske Bank (tech) | `danske` | EU/DK | https://danskebank.com/careers/tech | não verificado | TALVEZ (baixa) |
| Siteimprove | `siteimprove` | EU/DK | https://careers.siteimprove.com | não verificado | TALVEZ (baixa) |
| Novo Nordisk | `novonordisk` | EU/DK | https://www.novonordisk.com/careers/find-a-job.html | não verificado | TALVEZ (baixa); aviso de fraude em recrutamento |
| Unity | `unity` | NA/US | https://unity.com/careers | não verificado | TALVEZ (baixa); nenhuma vaga em Copenhagen vista |
| Zalando | `zalando` | EU/DE | https://jobs.zalando.com/ | não verificado | TALVEZ (média); muita vaga presencial |
| Booking.com | `booking` | EU/NL | https://jobs.booking.com | iCIMS (`external-workingatbooking.icims.com`); conta exigida | TALVEZ (média) |
| Adyen | `adyen` | EU/NL | https://careers.adyen.com | Greenhouse (`job-boards.greenhouse.io/adyen`) | TALVEZ (média); "office-first", sem remoto |
| Wise | `wise` | EU/GB | https://wise.jobs | SmartRecruiters (`smartr.me`); conta provável | TALVEZ (média); a única cláusula lida (SmartRecruiters, "use automatic means to access content or data from other users") fala de dados de outros usuários |
| N26, Celonis, Mollie, Personio, Revolut | `n26`, `celonis`, `mollie`, `personio`, `revolut` | EU | vários | não verificado | TALVEZ (baixa) |
| Zup | `zup` | SA/BR | https://www.zup.com.br/carreiras | Greenhouse (`boards.greenhouse.io/zupinnovation`) | TALVEZ (média); 3 vagas, todas remotas |
| QuintoAndar | `quintoandar` | SA/BR | https://carreiras.quintoandar.com.br | não verificado | TALVEZ (média); remoto só no Brasil |
| iFood, TOTVS, Mercado Libre | `ifood`, `totvs`, `mercadolibre` | SA | vários (404 ou 403 hoje) | não verificado | TALVEZ (baixa) |

**Descartadas ou adiadas:** Magalu (candidata pela 99jobs), Stone (Gupy com 0 vagas hoje), Creditas (404), Globo (domínio bloqueado). **Sem verificação:** Maersk, Vivino e Zendesk Copenhagen.

### O que as empresas ensinam sobre o modelo

- **O modelo deve ser a empresa, não o ATS.** Workday usa subdomínio por empresa e por servidor (`nvidia.wd5.myworkdayjobs.com`); um curinga `*.myworkdayjobs.com` abriria demais. Greenhouse, Lever e Ashby usam **caminho** por empresa (`/anthropic`), então casar só o domínio também abriria demais.
- **Cada empresa precisa de uma lista de domínios**, porque há aliases e redirecionamentos: `careers.google.com`, `careers.microsoft.com`, `boards.greenhouse.io`, `nubank.com.br/carreiras`.
- **Conta por empresa** (conhecimento prévio, não verificado): Workday, SuccessFactors, Taleo, iCIMS e Eightfold costumam exigir; Greenhouse, Lever e Ashby costumam aceitar candidatura sem conta.
- **Idioma é o principal filtro na Dinamarca:** Lunar e Netcompany pedem dinamarquês ou sueco em muitas vagas; Pleo, Templafy e Spotify operam em inglês.
- Poucos ATS concentram a maioria (Ashby, Greenhouse, Teamtailor, Lever), o que simplifica a automação.

### Corporativas "normais" (pedido do usuário: HP, Dell, SAP e parecidas)

| Empresa | chave | Reg/País | Carreiras | ATS / domínios | Veredito |
|---|---|---|---|---|---|
| Accenture | `accenture` | GLOBAL | https://www.accenture.com/dk-en/careers | Workday: `accenture.wd103.myworkdayjobs.com/AccentureCareers`; conta provável | INCLUIR (média) |
| LEGO Group | `lego` | EU/DK | https://www.lego.com/en-dk/careers/how-to-apply | Workday: `lego.wd103.myworkdayjobs.com`; **cria perfil na candidatura**; triagem assistida por IA na DK, UK e Boston | INCLUIR (média) |
| Ørsted | `orsted` | EU/DK | https://orsted.com/en/careers/vacancies-list | ATS não identificado; "Sign in or create an account" | INCLUIR (média) |
| SAP | `sap` | GLOBAL | https://jobs.sap.com/ | SuccessFactors (inferência); **parece migrar para `sap.com/en/jobs`, que deu 403**; 188 vagas na busca por Denmark, nenhuma listada na DK | TALVEZ (baixa) |
| Dell | `dell` | GLOBAL | `jobs.dell.com` redireciona para `enterpriseplatform.dell.com/hcmUI/CandidateExperience/` | Oracle HCM (inferência pelo caminho); escritório em Copenhagen | TALVEZ (baixa) |
| HP Inc | `hp` | GLOBAL | https://jobs.hp.com/ | `apply.hp.com`; ATS contraditório entre resumos (Eightfold ou outro); `careers.hp.com` não resolve | TALVEZ (baixa) |
| IBM | `ibm` | GLOBAL | https://www.ibm.com/careers | `careers.ibm.com`, `www.ibm.com/careers/search`; vagas da IBM Danmark em Holte (fonte secundária) | TALVEZ (baixa) |
| Cisco | `cisco` | GLOBAL | https://careers.cisco.com/ | Phenom (CDN `phenompeople.com`) | TALVEZ (média) |
| Nordea | `nordea` | EU/DK | https://careers.nordea.com/ | não identificado; vagas em `careers.nordea.com/job/...` | TALVEZ (média) |
| Maersk | `maersk` | EU/DK | `maersk.com/careers` deu erro de leitura | Workday como RH (via busca) | TALVEZ (baixa) |
| Capgemini, Ericsson, Grundfos, Carlsberg, Kyndryl | `capgemini`, `ericsson`, `grundfos`, `carlsberg`, `kyndryl` | vários | páginas de carreiras abriram ou redirecionaram | ATS não identificado | TALVEZ (baixa) |

**Sem leitura utilizável (403, DNS, SSL):** Oracle, Intel, Siemens, Nokia, Deloitte DK, Arla. **Não pesquisadas por limite de esforço, não por mérito:** Lenovo, Philips, Bosch, KPMG, EY, PwC, TCS, Infosys, Atos, CGI.

### Empresas "legais" (pedido do usuário: Canonical e parecidas)

| Empresa | chave | Reg/País | ATS (domínio de candidatura) | Remoto / elegibilidade | Veredito |
|---|---|---|---|---|---|
| **Canonical** | `canonical` | GLOBAL/GB | Greenhouse (`job-boards.greenhouse.io/canonical`) | totalmente distribuída, "any country"; 75+ vagas pela API; **limite de 1 vaga por 180 dias, ou 4 vagas diferentes**; processo de ~3 meses com avaliação escrita | INCLUIR (alta) |
| GitLab | `gitlab` | GLOBAL/US | Greenhouse (`job-boards.greenhouse.io/gitlab`) | all-remote; 150 vagas; vaga LATAM/Brasil vista | INCLUIR (alta) |
| Supabase | `supabase` | GLOBAL/US | Ashby (`jobs.ashbyhq.com/supabase`) | fully remote em 60+ países; contagens divergem (9 pela API, 53 no site) | INCLUIR (alta) |
| Grafana Labs | `grafanalabs` | GLOBAL/US | Greenhouse (`job-boards.greenhouse.io/grafanalabs`) | remote por país (NL, DE, UK vistos); 112 vagas | INCLUIR (média) |
| Elastic | `elastic` | GLOBAL/NL | Greenhouse (`jobs.elastic.co`) | "distributed by design", 40+ países | INCLUIR (média) |
| Linear | `linear` | GLOBAL/US | Ashby (`jobs.ashbyhq.com/linear`) | vagas "Europe, Remote"; 8 vagas | INCLUIR (média) |
| Hugging Face | `huggingface` | GLOBAL/FR | Workable (`apply.workable.com/huggingface`) | vagas "EMEA Remote"; 6 vagas | INCLUIR (média) |
| Automattic | `automattic` | GLOBAL/US | Greenhouse (board `automatticcareers`) | distribuída, 83 países; trial de até 20 horas; **declara avaliar candidaturas com ferramentas automatizadas e verificar identidade** | INCLUIR (média) |
| Mozilla | `mozilla` | GLOBAL/US | Greenhouse (embutido em `mozilla.org`) | remoto por país (FR, DE, UK, US); DK e Brasil não vistos | TALVEZ (média) |
| Sentry | `sentry` | GLOBAL/US | Ashby (`jobs.ashbyhq.com/sentry`) | política remota não verificada | TALVEZ (média) |
| Tailscale | `tailscale` | NA/US | Greenhouse (`job-boards.greenhouse.io/tailscale`) | só CA, US, UK | TALVEZ (média) |
| Mistral AI | `mistral` | EU/FR | Ashby (`jobs.ashbyhq.com/mistral.ai`) | escritórios europeus, sem DK | TALVEZ (média) |
| JetBrains | `jetbrains` | EU/CZ | Greenhouse EU (`job-boards.eu.greenhouse.io/jetbrains`) | híbrido, 15 escritórios | TALVEZ (média) |
| Doist (Todoist) | `doist` | GLOBAL/PT | não identificado | "work from anywhere"; sem confirmar vagas hoje | TALVEZ (média) |
| Red Hat | `redhat` | GLOBAL/US | Workday (`redhat.wd5.myworkdayjobs.com`); **exige conta** | remoto parcial | TALVEZ (média) |
| SUSE | `suse` | EU/DE | Workday (`suse.wd3.myworkdayjobs.com`); **exige conta** | não verificado | TALVEZ (baixa) |
| Cohere, Netlify, Prisma, Vercel | `cohere`, `netlify`, `prisma`, `vercel` | vários | Ashby, Greenhouse e não identificado | foco EUA ou poucas vagas | TALVEZ (baixa) |

**Descartadas:** 37signals (sem vagas hoje), HashiCorp (adquirida pela IBM em 2025), PlanetScale (só EUA), Zapier (só EUA, Canadá e Índia). **Sem leitura:** Docker (404), Fly.io (política remota não confirmada). **Não pesquisadas:** Buffer, Postman, Figma, Notion.

### Observações sobre empresas

- **Greenhouse domina** entre as "legais"; o resto usa Ashby ou Workable. Domínios a liberar por empresa: `job-boards.greenhouse.io`, `job-boards.eu.greenhouse.io`, `jobs.ashbyhq.com`, `apply.workable.com`, mais os subdomínios Workday exatos.
- **Dois tipos de limite que o app precisa respeitar:** a Canonical limita candidaturas por empresa (180 dias), e a Automattic avalia com ferramentas automatizadas. Candidatura em lote nessas duas é contraproducente.
- **Dinamarca:** nenhuma vaga amostrada das "legais" estava explicitamente lá; o caminho é "Remote EMEA/Europe".

## FREELANCE (`tipo = freelance`)

**Os termos proíbem automação quase sempre.** As cláusulas abaixo foram citadas por resumo de leitura; Upwork, Fiverr e Malt bloquearam a leitura (403). **Conclusão do agente: tratar todas como descoberta, com envio manual.**

| Plataforma | chave | Reg | Entrada | Modelo e custo ao freelancer | Brasil / DK | Veredito | Termos |
|---|---|---|---|---|---|---|---|
| Worksome | `worksome` | EU/DK | https://www.worksome.com | rede vetada; custo NÃO VERIFICADO | empresa dinamarquesa; aceite de brasileiros NÃO VERIFICADO | INCLUIR (média) | AUP v1 (2024-03-01) existe; cláusula NÃO LIDA |
| Malt | `malt` | EU | https://www.malt.com | marketplace; comissão 10%, 5% após 6 meses, 2% cliente próprio (fonte secundária; três revisões em 2026) | **Brasil só com empresa registrada**; DK NÃO VERIFICADO | INCLUIR (média) | NÃO VERIFICADO (403) |
| freelancermap | `freelancermap` | EU (DACH) | https://www.freelancermap.de | grátis (10 candidaturas/mês), Premium €13,99/mês, sem comissão | NÃO VERIFICADO; muitas vagas exigem alemão | INCLUIR (média) | NÃO VERIFICADO (404) |
| Lemon.io | `lemonio` | GLOBAL | https://lemon.io | rede vetada, 5 etapas; taxa não divulgada | **Brasil citado na página oficial** | INCLUIR (alta p/ BR) | **⚠ proíbe** (§20(K) e (L)); conta única |
| Toptal | `toptal` | GLOBAL | https://www.toptal.com | rede vetada, ~3% aceitos, 3 a 8 semanas | NÃO VERIFICADO | INCLUIR como descoberta (média) | **⚠ proíbe** ("page-scraper, deep-link, spider, robot") |
| Upwork | `upwork` | GLOBAL | https://www.upwork.com/ab/account-security/login (por busca; 403) | 0 a 15% por contrato + Connects (~US$0,15, 2 a 16 por proposta) | amplamente usado | TALVEZ (média) | **⚠ proíbe** (via busca); auto-bidders banidos, inclusive por fingerprint |
| Freelancer.com | `freelancercom` | GLOBAL | https://www.freelancer.com/login | lances; 10% por projeto | conta aberta globalmente | TALVEZ (média) | **⚠ proíbe** (§33, vale para a API) |
| Turing | `turing` | GLOBAL | https://www.turing.com | rede vetada | NÃO VERIFICADO | TALVEZ (média) | **⚠ proíbe** (§6(b)); múltiplos perfis = fraude |
| Contra | `contra` | GLOBAL | https://contra.com | 0% de comissão; Pro US$29/mês | NÃO VERIFICADO | TALVEZ (média) | **⚠ proíbe** ("multiple accounts, automated tools") |
| Crossover | `crossover` | GLOBAL | https://www.crossover.com | emprego remoto full-time com testes e monitoramento | NÃO VERIFICADO | TALVEZ (média) | NÃO VERIFICADO |
| Braintrust, Andela, Malt Strategy (ex-Comatch), Freelance.de | `braintrust`, `andela`, `maltstrategy`, `freelancede` | vários | vários | rede vetada ou DACH | NÃO VERIFICADO | TALVEZ (baixa) | parcial ou NÃO VERIFICADO |

**Curadoria do freelance (pedido do usuário, 2026-10-14): "as melhores, de preferência fora da SA, para ganhar em dólar".** Ordem abaixo é **opinião minha**, montada só com o que os agentes leram (nada novo foi pesquisado); custo, moeda e aceite de brasileiro seguem em parte NÃO VERIFICADO.

| Faixa | Plataforma | Por quê | Ressalvas |
|---|---|---|---|
| 1 | **Lemon.io** | rede vetada para devs, **cita o Brasil na página oficial**, clientes em dólar (moeda não confirmada), pagamento por Payoneer ou Wise | funil único de 5 etapas (~1,2% aceitos, fonte secundária); proíbe bots |
| 1 | **Toptal** | rede vetada de alto nível, taxa horária definida por você, cobrança pela Toptal | ~3% aceitos, 3 a 8 semanas; proíbe robôs; aceite de brasileiro NÃO VERIFICADO |
| 1 | **Upwork** | maior volume, contratos em dólar | 0 a 15% + Connects (~US$0,15 cada); **maior risco de banimento por automação**, usar só para descobrir e propor à mão |
| 2 | **Contra** | 0% de comissão sobre projetos, global | Pro US$29/mês; volume depende da demanda; proíbe automação |
| 2 | **Braintrust** | sem taxa ao talento (a confirmar nos termos), rede vetada | um único perfil permitido; volume incerto |
| 2 | **Turing** | rede vetada, vagas de empresas dos EUA | longas esperas sem match (Trustpilot misto); proíbe automação |
| 3 | **Crossover** | pagamento em dólar, emprego remoto full-time | é mais emprego que freelance; testes e monitoramento de produtividade |
| 3 | **Arc.dev** | rede vetada, foco em clientes dos EUA | proíbe automação; vetting demorado |
| 3 | **Freelancer.com** | aberto, dólar | 10% por projeto; saturado e com spam de propostas |
| Europa (euro) | **Malt**, **Worksome**, **freelancermap** | úteis se quiser a Europa/Dinamarca | **Malt só aceita brasileiro com empresa registrada**; freelancermap pede alemão em muitos casos; não pagam em dólar |

**Fora do seed:** Fiverr, Gun.io, Codementor, Twago, Mercor, Outlier, 99Freelas e Workana. **Recomendação:** no app, todas entram como `papel = descoberta`, com o envio de propostas feito por você, porque quase todas proíbem automação.

**Decisão do usuário (2026-10-14): no freelance, só plataformas de fora da América do Sul** ("da gringa, nenhuma daqui da SA"). Por isso **99Freelas** (BR) e **Workana** ficam fora do seed; Lemon.io e as demais globais ou europeias continuam.

**Descartadas:** 99Freelas e Workana (por decisão do usuário, ver acima), Fiverr (vende "gigs", não propõe a vagas), Gun.io (convite, focado em EUA e Canadá), Codementor (mentoria), Twago (adquirida pela Randstad em 2016), Comatch como marca (virou Malt Strategy em 2022), **Mercor e Outlier** (processos, vazamento de dados e retenção de pagamento, por fontes secundárias). **Sem confirmação:** Hired.com, Konsulentbørsen/Freelance.dk (só Edora e Freelance.nl apareceram).

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

## Próximas etapas

- **Passada de verificação com navegador real:** termos de uso (LinkedIn, Indeed, Glassdoor, Workday, SuccessFactors, Greenhouse, Ashby), URLs de login marcadas NÃO VERIFICADO e o estado das empresas que mudaram de domínio (SAP, Dell, HP).
- **Curadoria:** reduzir as ~190 entradas listadas para ~100 com o usuário, decidir o risco dos termos e fechar o seed na spec.
- **Lacunas:** África e Oriente Médio, China e Hong Kong, Coreia, PCD e mulheres em tecnologia, e as corporativas não pesquisadas (Lenovo, Philips, Bosch, KPMG, EY, PwC, TCS, Infosys, Atos, CGI).
