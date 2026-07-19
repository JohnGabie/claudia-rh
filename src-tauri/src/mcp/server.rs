// rmcp glue: exposes the tools from `dispatch` over MCP stdio. Thin layer —
// schemas come from the Parameters structs, behavior lives in mod.rs/tools.rs.

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerInfo,
};
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt};

use super::{dispatch, McpConfig};

#[derive(Clone)]
struct ClaudiaMcp {
    cfg: Arc<McpConfig>,
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

impl ClaudiaMcp {
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
}

#[tool_handler]
impl ServerHandler for ClaudiaMcp {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.server_info = Implementation::new("claudia-rh", env!("CARGO_PKG_VERSION"));
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.instructions = Some(
            "Ferramentas da Claudia RH: use update_profile para gravar o perfil \
             (nunca escreva candidate_base.yaml diretamente) e \
             list_pendencias/close_pendencia para gerir pendências."
                .into(),
        );
        info
    }
}

pub fn run(cfg: McpConfig) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let service = ClaudiaMcp { cfg: Arc::new(cfg) }
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|e| e.to_string())?;
        service.waiting().await.map_err(|e| e.to_string())?;
        Ok(())
    })
}
