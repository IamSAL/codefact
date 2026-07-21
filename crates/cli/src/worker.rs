//! The codefact iii worker, run in-process by `codefact __worker` (spawned by
//! `codefact start`). Wires the engine's triggers/functions to `codefact-core`,
//! backing the graph with iii-state.

use std::sync::Arc;
use std::time::Duration;

use codefact_core::config::Config;
use codefact_core::engine::ClaudeEngine;
use codefact_core::graph::Graph;
use codefact_core::miner::{self, SliceInput};
use codefact_core::secrets::Secrets;
use codefact_core::sender::TelegramSender;
use codefact_core::{paths, run_emit, run_mine};

use iii_sdk::builtin_triggers::{CronTriggerConfig, HttpMethod, HttpTriggerConfig, IIITrigger};
use iii_sdk::errors::Error;
use iii_sdk::protocol::TriggerRequest;
use iii_sdk::{IIIClient, InitOptions, RegisterFunction, register_worker};
use serde_json::{Value, json};

use crate::store::IiiStore;

struct Ctx {
    client: IIIClient,
    config: Config,
    secrets: Option<Secrets>,
}

impl Ctx {
    fn store(&self) -> IiiStore {
        IiiStore::new(self.client.clone())
    }

    fn engine(&self) -> ClaudeEngine {
        let allowed = self
            .config
            .repos
            .iter()
            .filter_map(|r| std::fs::canonicalize(&r.path).ok())
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        ClaudeEngine {
            bin: self.config.engine.bin.clone(),
            allowed_tools: self.config.engine.allowed_tools.clone(),
            timeout: Duration::from_secs(self.config.engine.timeout_secs),
            allowed_repo_paths: allowed,
        }
    }
}

/// Connect to the engine, register functions + triggers, and run until Ctrl-C.
pub async fn run() -> anyhow::Result<()> {
    let url = std::env::var("III_URL").unwrap_or_else(|_| "ws://localhost:49134".to_string());

    let config_path = std::env::var("codefact_CONFIG")
        .map(std::path::PathBuf::from)
        .or_else(|_| paths::config_path())?;
    let config = Config::load(&config_path)?;

    let secrets_path = std::env::var("codefact_SECRETS")
        .map(std::path::PathBuf::from)
        .or_else(|_| paths::secrets_path())?;
    let secrets = Secrets::load(&secrets_path).ok();

    let client = register_worker(&url, InitOptions::default());
    let ctx = Arc::new(Ctx {
        client: client.clone(),
        config,
        secrets,
    });

    register(&client, &ctx, "codefact::status", status);
    register(&client, &ctx, "codefact::mine", mine);
    register(&client, &ctx, "codefact::emit", emit);
    register(&client, &ctx, "codefact::tick", tick);

    let mut _handles = Vec::new();
    for expr in codefact_core::config::times_to_cron(&ctx.config.schedule.times)? {
        _handles.push(
            client.register_trigger(
                IIITrigger::Cron(CronTriggerConfig::new(expr)).for_function("codefact::tick"),
            )?,
        );
    }
    _handles.push(
        client.register_trigger(
            IIITrigger::Http(HttpTriggerConfig::new("/emit").method(HttpMethod::Post))
                .for_function("codefact::emit"),
        )?,
    );
    _handles.push(
        client.register_trigger(
            IIITrigger::Http(HttpTriggerConfig::new("/mine").method(HttpMethod::Post))
                .for_function("codefact::mine"),
        )?,
    );

    println!("codefact worker ready (engine: {url})");
    tokio::signal::ctrl_c().await?;
    client.shutdown();
    Ok(())
}

fn register<F, Fut>(client: &IIIClient, ctx: &Arc<Ctx>, id: &str, f: F)
where
    F: Fn(Arc<Ctx>, Value) -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = Result<Value, Error>> + Send + 'static,
{
    let ctx = ctx.clone();
    client.register_function(
        id,
        RegisterFunction::new_async(move |req| {
            let ctx = ctx.clone();
            let f = f.clone();
            async move { f(ctx, req).await }
        }),
    );
}

fn handler_err(e: anyhow::Error) -> Error {
    Error::Handler(e.to_string())
}

async fn status(ctx: Arc<Ctx>, _req: Value) -> Result<Value, Error> {
    Ok(json!({
        "ok": true,
        "worker": "codefact",
        "version": env!("CARGO_PKG_VERSION"),
        "repos": ctx.config.repos.len(),
        "times": ctx.config.schedule.times,
        "telegram_configured": ctx.secrets.is_some(),
    }))
}

async fn mine(ctx: Arc<Ctx>, _req: Value) -> Result<Value, Error> {
    let Some(repo) = ctx.config.repos.first().cloned() else {
        return Ok(json!({ "mined_nodes": 0, "reason": "no repos configured" }));
    };
    let store = ctx.store();

    let covered = Graph::new(&store, paths::repo_id(&repo.path))
        .covered_paths()
        .await
        .map_err(handler_err)?;
    let changed = miner::git_changed(&repo.path);
    let uncovered: Vec<String> = miner::list_repo_files(&repo.path)
        .into_iter()
        .filter(|f| !covered.contains(f))
        .collect();
    let files = miner::select_slice(&SliceInput {
        changed,
        uncovered,
        stale: Vec::new(),
        limit: ctx.config.engine.slice_files,
    });

    let engine = ctx.engine();
    let n = run_mine(&store, &engine, &repo, files)
        .await
        .map_err(handler_err)?;
    Ok(json!({ "mined_nodes": n }))
}

async fn emit(ctx: Arc<Ctx>, _req: Value) -> Result<Value, Error> {
    let Some(repo) = ctx.config.repos.first().cloned() else {
        return Ok(json!({ "emitted": false, "reason": "no repos configured" }));
    };
    let Some(secrets) = ctx.secrets.clone() else {
        return Ok(json!({ "emitted": false, "reason": "telegram not configured" }));
    };
    let store = ctx.store();
    let sender = TelegramSender {
        bot_token: secrets.telegram_bot_token,
        chat_id: secrets.telegram_chat_id,
    };
    match run_emit(&store, &sender, &repo).await.map_err(handler_err)? {
        Some(text) => Ok(json!({ "emitted": true, "text": text })),
        None => Ok(json!({ "emitted": false, "reason": "nothing new to emit" })),
    }
}

async fn tick(ctx: Arc<Ctx>, _req: Value) -> Result<Value, Error> {
    let mined = ctx
        .client
        .trigger(TriggerRequest {
            function_id: "codefact::mine".to_string(),
            payload: json!({}),
            action: None,
            timeout_ms: Some(300_000),
        })
        .await?;
    let emitted = ctx
        .client
        .trigger(TriggerRequest {
            function_id: "codefact::emit".to_string(),
            payload: json!({}),
            action: None,
            timeout_ms: None,
        })
        .await?;
    Ok(json!({ "ticked": true, "mine": mined, "emit": emitted }))
}
