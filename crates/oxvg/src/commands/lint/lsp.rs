use oxvg_ast::{
    arena::Allocator,
    node::{Ranges, Ref},
    parse::roxmltree::{parse_with_options, ParsingOptions},
    visitor::Visitor,
};
use oxvg_lint::error::Error;
use oxvg_lint::{Rules, Severity};
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::lsp_types::notification::PublishDiagnostics;
#[allow(clippy::wildcard_imports)]
use tower_lsp_server::{lsp_types::*, UriExt};
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    rules: Rules,
}

static NAME: &str = "oxvg lint";

fn error_to_diagnostic(
    Error {
        problem,
        severity,
        range,
        help,
    }: Error,
    source: &str,
    uri: &Uri,
) -> Diagnostic {
    let range = match range {
        Some(range) => {
            let start = Ranges::to_line_and_col(range.start, source.as_bytes());
            let end = Ranges::to_line_and_col(range.end, source.as_bytes());
            Range {
                start: Position::new(start.0 as u32, start.1 as u32),
                end: Position::new(end.0 as u32, end.1 as u32),
            }
        }
        None => Range::default(),
    };
    Diagnostic {
        range,
        severity: match severity {
            Severity::Off => None,
            Severity::Warn => Some(DiagnosticSeverity::WARNING),
            Severity::Error => Some(DiagnosticSeverity::ERROR),
        },
        source: Some(NAME.into()),
        message: problem.to_string(),
        related_information: help.map(|help| {
            vec![DiagnosticRelatedInformation {
                location: Location {
                    uri: uri.clone(),
                    range,
                },
                message: help,
            }]
        }),
        ..Diagnostic::default()
    }
}

impl Backend {
    fn analyse<'input, 'arena>(
        &self,
        dom: Ref<'input, 'arena>,
        allocator: Allocator<'input, 'arena>,
        source: &str,
        uri: &Uri,
    ) -> std::result::Result<(), Vec<Diagnostic>> {
        let path = uri.to_file_path().map(|p| p.to_path_buf());
        let Err(diagnostics) = self.rules.start_with_path(dom, allocator, path) else {
            return Ok(());
        };
        Err(diagnostics
            .into_iter()
            .map(|error| error_to_diagnostic(error, source, uri))
            .collect())
    }

    async fn lint(&self, uri: &Uri, source: &str) {
        let uri = uri.clone();
        let Ok(Err(diagnostics)) = parse_with_options(
            source,
            ParsingOptions {
                allow_dtd: true,
                ..ParsingOptions::default()
            },
            |dom, allocator| self.analyse(dom, allocator, source, &uri),
        ) else {
            return;
        };

        let result = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        };

        self.client
            .send_notification::<PublishDiagnostics>(result)
            .await;
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: String::from(NAME),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                    },
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some(String::from(NAME)),
                        inter_file_dependencies: false,
                        workspace_diagnostics: true,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: None,
                        },
                    },
                )),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.lint(&params.text_document.uri, &params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let Some(last) = params.content_changes.last() else {
            return;
        };
        self.lint(&params.text_document.uri, &last.text).await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub async fn serve(rules: Rules) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client, rules });
    Server::new(stdin, stdout, socket).serve(service).await;
}
