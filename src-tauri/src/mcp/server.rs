// rmcp glue: exposes the tools from `dispatch` over MCP stdio. Thin layer —
// schemas come from the Parameters structs, behavior lives in mod.rs/tools.rs.

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerInfo,
};
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt};

use super::{dispatch, McpConfig};
use super::tools::browser::{BrowserTools, DbGuardContext, NoSink, ToolOutput};

#[derive(Clone)]
struct ClaudiaMcp {
    cfg: Arc<McpConfig>,
    browser: Arc<BrowserTools>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct UpdateProfileArgs {
    /// Conteúdo YAML completo do candidate_base.yaml (o arquivo inteiro, não um fragmento)
    yaml: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ClosePendenciaArgs {
    /// ID numérico da pendência (ver list_pendencias)
    id: i64,
    /// Motivo/resolução curta, ex.: "Salário definido: 8000 BRL"
    resolucao: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ProposeProfileChangeArgs {
    /// A pergunta ou mudança que o perfil precisa, em uma frase
    pergunta: String,
    /// Porque surgiu: a vaga, o formulário, o padrão observado
    contexto: Option<String>,
    /// ID da vaga que motivou a proposta; omita quando a questão for geral
    vaga_id: Option<i64>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct VagaIdArgs {
    /// ID da vaga
    vaga_id: i64,
    /// Resolução aplicada (só usado por close_pendencias_vaga)
    resolucao: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct RegisterVagaArgs {
    /// Título do cargo
    titulo: String,
    /// Nome da empresa
    empresa: String,
    /// Plataforma onde foi descoberta (linkedin, linkedin_rede, indeed, site_empresa, …)
    plataforma: String,
    /// URL única da vaga
    url: String,
    /// Localização (cidade/país), se visível
    localizacao: Option<String>,
    /// remoto | hibrido | presencial, se visível
    modelo_trabalho: Option<String>,
    /// Nome da conexão que compartilhou a vaga (descoberta via rede)
    fonte_conexao: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct UpdateVagaStatusArgs {
    /// ID da vaga
    vaga_id: i64,
    /// Novo status: descoberta | analisada | candidatando | aplicada | pulada | pendente_revisao
    status: String,
    /// Para 'analisada': resumo do match. Para 'pulada'/'pendente_revisao': motivo
    detalhe: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct RegisterCandidaturaArgs {
    /// ID da vaga aplicada
    vaga_id: i64,
    /// Caminho da pasta com os arquivos gerados (CV, carta…)
    pasta_arquivos: String,
    /// Método de candidatura: formulario | email | easy_apply | outro
    metodo: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CreatePendenciaArgs {
    /// ID da vaga travada
    vaga_id: i64,
    /// Categoria da pausa (salario, visto, idioma, dados_pessoais, outra)
    categoria: String,
    /// Descrição legível do que travou a candidatura
    descricao: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BrowserOpenArgs {
    /// URL completa, http ou https
    url: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BrowserReadArgs {
    /// Limita a leitura ao elemento cujo papel ou nome contém este texto, ex.: "dialog" para só o modal
    scope: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BrowserClickArgs {
    /// Referência do elemento, ex.: "e12", vinda de browser_read
    reference: String,
    /// Só para botões que enviam a candidatura: ID da vaga (precisa ser a vaga ativa, em 'candidatando')
    confirm_submit: Option<i64>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BrowserTypeArgs {
    /// Referência do campo, ex.: "e5", vinda de browser_read
    reference: String,
    /// Texto a escrever
    text: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BrowserKeyArgs {
    /// Enter, Tab, Escape ou Space
    key: String,
    /// Só se o foco estiver num botão de enviar candidatura (ou num campo de um formulário que a envia): ID da vaga ativa
    confirm_submit: Option<i64>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BrowserCropArgs {
    /// Canto superior esquerdo do recorte, em pixels CSS
    x: f64,
    y: f64,
    /// Largura e altura do recorte, em pixels CSS
    width: f64,
    height: f64,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BrowserClickAtArgs {
    x: f64,
    y: f64,
    /// Só para botões que enviam a candidatura: ID da vaga ativa
    confirm_submit: Option<i64>,
}

impl ClaudiaMcp {
    fn browser_text(&self, tool: &str, r: Result<String, String>) -> Result<CallToolResult, rmcp::ErrorData> {
        match &r {
            Ok(_) => crate::diagnostics::emit_event("mcp", "tool_ok", None, tool),
            Err(e) => crate::diagnostics::emit_event("mcp", "tool_err", None, &format!("{tool}: {e}")),
        }
        match r {
            Ok(m) => Ok(CallToolResult::success(vec![ContentBlock::text(m)])),
            // Tool-level error: the model sees the message and can self-correct.
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(e)])),
        }
    }

    fn call(&self, tool: &str, args: serde_json::Value) -> Result<CallToolResult, rmcp::ErrorData> {
        match dispatch(&self.cfg, tool, &args) {
            Ok(msg) => Ok(CallToolResult::success(vec![ContentBlock::text(msg)])),
            // Tool-level error: the model sees the message and can self-correct.
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(e)])),
        }
    }
}

#[tool_router]
impl ClaudiaMcp {
    #[tool(
        description = "Atualiza o perfil do candidato. Recebe o candidate_base.yaml COMPLETO, valida contra o schema e grava no local correto. Se o YAML for inválido, devolve o erro de validação — corrija e chame de novo. Use SEMPRE esta ferramenta em vez de escrever o arquivo diretamente."
    )]
    async fn update_profile(
        &self,
        Parameters(args): Parameters<UpdateProfileArgs>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call("update_profile", serde_json::json!({ "yaml": args.yaml }))
    }

    #[tool(
        description = "Regista uma mudança que o perfil do candidato precisa, para o usuário rever mais tarde. Use quando perceber que falta um dado no perfil ou que uma resposta se repete entre vagas. Omita vaga_id quando a questão não for sobre uma vaga específica. Esta é a forma correta de propor mudanças ao perfil durante a sessão de execução — update_profile não funciona aí."
    )]
    async fn propose_profile_change(
        &self,
        Parameters(args): Parameters<ProposeProfileChangeArgs>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call(
            "propose_profile_change",
            serde_json::json!({
                "pergunta": args.pergunta,
                "contexto": args.contexto,
                "vaga_id": args.vaga_id,
            }),
        )
    }

    #[tool(
        description = "Fecha (marca como resolvida) uma pendência do sistema pelo ID. Use list_pendencias antes para obter os IDs atuais."
    )]
    async fn close_pendencia(
        &self,
        Parameters(args): Parameters<ClosePendenciaArgs>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call(
            "close_pendencia",
            serde_json::json!({ "id": args.id, "resolucao": args.resolucao }),
        )
    }

    #[tool(description = "Lista as pendências abertas do sistema, com os seus IDs.")]
    async fn list_pendencias(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call("list_pendencias", serde_json::json!({}))
    }

    #[tool(
        description = "Devolve o candidate_base.yaml COMPLETO do candidato: dados pessoais, experiência, projetos, formação, competências e idiomas. O perfil NÃO está no prompt — chame esta ferramenta antes de avaliar uma vaga ou gerar currículo/carta. Nunca invente nem assuma dados do candidato."
    )]
    async fn get_candidate_profile(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call("get_candidate_profile", serde_json::json!({}))
    }

    #[tool(
        description = "Devolve o search_variants.yaml: os termos e filtros de pesquisa que o candidato definiu. Chame antes de começar a procurar vagas."
    )]
    async fn get_search_variants(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call("get_search_variants", serde_json::json!({}))
    }

    #[tool(
        description = "Devolve o strategy.md: as preferências do candidato sobre a que vagas se candidatar e quais evitar. Chame antes de decidir se uma vaga vale a pena."
    )]
    async fn get_strategy(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call("get_strategy", serde_json::json!({}))
    }

    #[tool(
        description = "Devolve o estado recente da sessão: candidaturas de hoje e da semana, pendências por resolver e vagas puladas recentemente. Recalculado a cada chamada — chame de novo depois de se candidatar para ver os contadores atualizados."
    )]
    async fn get_memory_summary(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call("get_memory_summary", serde_json::json!({}))
    }

    #[tool(
        description = "Fecha TODAS as pendências abertas de uma vaga (usar ao terminar uma vaga que estava em pendente_revisao). Preserva o texto de resolução que o usuário já tiver escrito."
    )]
    async fn close_pendencias_vaga(
        &self,
        Parameters(args): Parameters<VagaIdArgs>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call(
            "close_pendencias_vaga",
            serde_json::json!({ "vaga_id": args.vaga_id, "resolucao": args.resolucao }),
        )
    }

    #[tool(
        description = "Lê a pendência aberta mais recente de uma vaga, incluindo a resolução escrita pelo usuário (se houver). Chamar ANTES de retomar uma vaga em pendente_revisao."
    )]
    async fn get_pendencia_vaga(
        &self,
        Parameters(args): Parameters<VagaIdArgs>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call("get_pendencia_vaga", serde_json::json!({ "vaga_id": args.vaga_id }))
    }

    #[tool(
        description = "Registra uma vaga recém-descoberta (status inicial: descoberta). URLs duplicadas não são inseridas de novo — a resposta indica o ID existente."
    )]
    async fn register_vaga(
        &self,
        Parameters(args): Parameters<RegisterVagaArgs>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call(
            "register_vaga",
            serde_json::json!({
                "titulo": args.titulo, "empresa": args.empresa, "plataforma": args.plataforma,
                "url": args.url, "localizacao": args.localizacao,
                "modelo_trabalho": args.modelo_trabalho, "fonte_conexao": args.fonte_conexao,
            }),
        )
    }

    #[tool(
        description = "Avança o status de uma vaga (descoberta → analisada → candidatando → aplicada / pulada / pendente_revisao). Use 'detalhe' para o resumo do match (analisada) ou o motivo (pulada). A interface atualiza sozinha."
    )]
    async fn update_vaga_status(
        &self,
        Parameters(args): Parameters<UpdateVagaStatusArgs>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call(
            "update_vaga_status",
            serde_json::json!({ "vaga_id": args.vaga_id, "status": args.status, "detalhe": args.detalhe }),
        )
    }

    #[tool(
        description = "Registra uma candidatura enviada com sucesso e marca a vaga como 'aplicada' (transação única). Chamar exatamente uma vez por candidatura concluída."
    )]
    async fn register_candidatura(
        &self,
        Parameters(args): Parameters<RegisterCandidaturaArgs>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call(
            "register_candidatura",
            serde_json::json!({
                "vaga_id": args.vaga_id, "pasta_arquivos": args.pasta_arquivos, "metodo": args.metodo,
            }),
        )
    }

    #[tool(
        description = "Abre uma pendência (condição de pausa que exige o usuário) e marca a vaga como pendente_revisao (transação única)."
    )]
    async fn create_pendencia(
        &self,
        Parameters(args): Parameters<CreatePendenciaArgs>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call(
            "create_pendencia",
            serde_json::json!({
                "vaga_id": args.vaga_id, "categoria": args.categoria, "descricao": args.descricao,
            }),
        )
    }

    #[tool(
        description = "Abre uma URL (http ou https) no browser da Claudia, numa aba própria do agente. Depois chame browser_read para ver a página."
    )]
    async fn browser_open(&self, Parameters(a): Parameters<BrowserOpenArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_open", self.browser.open(&a.url).await)
    }

    #[tool(
        description = "Lista os elementos interativos da página, cada um com uma ref (e1, e2, ...) para usar em browser_click e browser_type. 'scope' limita a um bloco, ex.: \"dialog\" para só o modal. As refs valem só até a página mudar."
    )]
    async fn browser_read(&self, Parameters(a): Parameters<BrowserReadArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_read", self.browser.read(a.scope.as_deref()).await)
    }

    #[tool(
        description = "Clica no elemento pela ref de browser_read. Botões que enviam a candidatura são RECUSADOS a menos que confirm_submit traga o ID da vaga ativa (em 'candidatando'), dentro do orçamento diário e uma só vez por vaga."
    )]
    async fn browser_click(&self, Parameters(a): Parameters<BrowserClickArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_click", self.browser.click(&a.reference, a.confirm_submit).await)
    }

    #[tool(description = "Escreve texto no campo indicado pela ref de browser_read.")]
    async fn browser_type(&self, Parameters(a): Parameters<BrowserTypeArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_type", self.browser.type_text(&a.reference, &a.text).await)
    }

    #[tool(
        description = "Pressiona uma tecla (Enter, Tab, Escape ou Space) no elemento em foco. Enter ou Space sobre um botão de enviar candidatura, ou Enter num campo de um formulário que a envia, passam pela mesma guarda do clique."
    )]
    async fn browser_key(&self, Parameters(a): Parameters<BrowserKeyArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_key", self.browser.key(&a.key, a.confirm_submit).await)
    }

    #[tool(
        description = "Devolve uma imagem de um recorte da página (x, y, largura, altura em pixels CSS). Use quando a árvore de browser_read não mostrar o que você precisa."
    )]
    async fn browser_crop(&self, Parameters(a): Parameters<BrowserCropArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.browser.crop(a.x, a.y, a.width, a.height).await {
            Ok(ToolOutput::Image { data_base64, mime }) => {
                crate::diagnostics::emit_event("mcp", "tool_ok", None, "browser_crop");
                Ok(CallToolResult::success(vec![ContentBlock::image(data_base64, mime)]))
            }
            Ok(ToolOutput::Text(t)) => self.browser_text("browser_crop", Ok(t)),
            Err(e) => self.browser_text("browser_crop", Err(e)),
        }
    }

    #[tool(
        description = "Clica numa coordenada da página, só quando a árvore de browser_read não expõe o elemento. A guarda de envio olha o elemento que está sob o ponto."
    )]
    async fn browser_click_at(&self, Parameters(a): Parameters<BrowserClickAtArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_click_at", self.browser.click_at(a.x, a.y, a.confirm_submit).await)
    }
}

#[tool_handler]
impl ServerHandler for ClaudiaMcp {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.server_info = Implementation::new("claudia-rh", env!("CARGO_PKG_VERSION"));
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.instructions = Some(
            "Ferramentas da Claudia RH. O perfil do candidato NÃO está no prompt: \
             leia-o com get_candidate_profile, e leia get_search_variants e \
             get_strategy antes de procurar ou avaliar vagas. Nunca invente dados \
             do candidato — se uma leitura falhar, pare e reporte em vez de assumir. \
             Use update_profile para gravar o perfil (nunca escreva \
             candidate_base.yaml diretamente) e list_pendencias/close_pendencia \
             para gerir pendências."
                .into(),
        );
        info
    }
}

pub fn run(cfg: McpConfig) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        // Without --browser-state the tools still exist and answer "ainda não está pronto".
        let state_file = cfg.browser_state.clone().unwrap_or_else(|| cfg.data_dir.join("browser-state-not-configured.json"));
        let guard = Arc::new(DbGuardContext::new(cfg.data_dir.clone()));
        // With --local-data-dir the dev-mode Activity log is wired in; it records only if the switch is on.
        let sink: Arc<dyn super::tools::browser::ActionSink> = match &cfg.local_data_dir {
            Some(local) => Arc::new(crate::activity_log::sink::LogSink::new(cfg.data_dir.clone(), local.clone())),
            None => Arc::new(NoSink),
        };
        let browser = Arc::new(BrowserTools::new(state_file, guard, sink));
        let service = ClaudiaMcp { cfg: Arc::new(cfg), browser }
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|e| e.to_string())?;
        service.waiting().await.map_err(|e| e.to_string())?;
        Ok(())
    })
}
