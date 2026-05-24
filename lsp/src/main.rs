use tower_lsp_server::{LspService, Server};

mod server;
mod parser;
mod ast;
mod semantic;
mod config;
mod include_resolver;
mod commands;
mod completion;
mod hover;
mod goto;
mod symbols;
mod references;
mod rename;
mod formatting;
mod diagnostics;

use server::Backend;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    log::info!("lammps-lsp starting");

    let (service, socket) = LspService::new(|client| Backend::new(client));
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    Server::new(stdin, stdout, socket).serve(service).await;

    log::info!("lammps-lsp shutting down");
    Ok(())
}
