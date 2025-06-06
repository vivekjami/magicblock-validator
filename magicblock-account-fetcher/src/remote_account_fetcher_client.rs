use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, Mutex},
};

use conjunto_transwise::AccountChainSnapshotShared;
use futures_util::{
    future::{ready, BoxFuture},
    FutureExt,
};
use solana_sdk::{clock::Slot, pubkey::Pubkey};
use tokio::sync::{mpsc::UnboundedSender, oneshot::channel};

use crate::{
    AccountFetcher, AccountFetcherError, AccountFetcherListeners,
    AccountFetcherResult, RemoteAccountFetcherWorker,
};

pub struct RemoteAccountFetcherClient {
    fetch_request_sender: UnboundedSender<(Pubkey, Option<Slot>)>,
    fetch_listeners: Arc<Mutex<HashMap<Pubkey, AccountFetcherListeners>>>,
}

impl RemoteAccountFetcherClient {
    pub fn new(worker: &RemoteAccountFetcherWorker) -> Self {
        Self {
            fetch_request_sender: worker.get_fetch_request_sender(),
            fetch_listeners: worker.get_fetch_listeners(),
        }
    }
}

impl AccountFetcher for RemoteAccountFetcherClient {
    fn fetch_account_chain_snapshot(
        &self,
        pubkey: &Pubkey,
        min_context_slot: Option<Slot>,
    ) -> BoxFuture<AccountFetcherResult<AccountChainSnapshotShared>> {
        let (should_request_fetch, receiver) = match self
            .fetch_listeners
            .lock()
            .expect("RwLock of RemoteAccountFetcherClient.fetch_listeners is poisoned")
            .entry(*pubkey)
        {
            Entry::Vacant(entry) => {
                let (sender, receiver) = channel();
                entry.insert(vec![sender]);
                (true, receiver)
            }
            Entry::Occupied(mut entry) => {
                let (sender, receiver) = channel();
                entry.get_mut().push(sender);
                (false, receiver)
            }
        };
        // track the number of pending clones, might be helpful to detect memory leaks
        magicblock_metrics::metrics::inc_pending_clone_requests();
        if should_request_fetch {
            if let Err(error) =
                self.fetch_request_sender.send((*pubkey, min_context_slot))
            {
                return Box::pin(ready(Err(AccountFetcherError::SendError(
                    error,
                ))));
            }
        }
        Box::pin(receiver.map(|received| {
            magicblock_metrics::metrics::dec_pending_clone_requests();
            match received {
                Ok(result) => result,
                Err(error) => Err(AccountFetcherError::RecvError(error)),
            }
        }))
    }
}
