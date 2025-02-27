use super::sentry_status_provider::SentryStatusProvider;
use crate::{
    downloader::headers::downloader::{DownloaderReport, DownloaderRunState},
    kv,
    models::BlockNumber,
    sentry::{chain_config::ChainConfig, sentry_client_reactor::SentryClientReactorShared},
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct Downloader {
    headers_downloader: super::headers::downloader::Downloader,
    sentry_status_provider: SentryStatusProvider,
}

impl Downloader {
    pub fn new(
        chain_config: ChainConfig,
        mem_limit: usize,
        sentry: SentryClientReactorShared,
        sentry_status_provider: SentryStatusProvider,
    ) -> anyhow::Result<Self> {
        let headers_downloader =
            super::headers::downloader::Downloader::new(chain_config, mem_limit, sentry)?;

        let instance = Self {
            headers_downloader,
            sentry_status_provider,
        };
        Ok(instance)
    }

    pub async fn run<'downloader, 'db: 'downloader, RwTx: kv::traits::MutableTransaction<'db>>(
        &'downloader self,
        db_transaction: &'downloader RwTx,
        start_block_num: BlockNumber,
        max_blocks_count: usize,
        previous_run_state: Option<DownloaderRunState>,
    ) -> anyhow::Result<DownloaderReport> {
        self.sentry_status_provider.update(db_transaction).await?;

        let mut ui_system = crate::downloader::ui_system::UISystem::new();
        ui_system.start()?;
        let ui_system = Arc::new(Mutex::new(ui_system));

        let report = self
            .headers_downloader
            .run::<RwTx>(
                db_transaction,
                start_block_num,
                max_blocks_count,
                previous_run_state,
                ui_system.clone(),
            )
            .await?;

        ui_system.try_lock()?.stop().await?;

        Ok(report)
    }
}
