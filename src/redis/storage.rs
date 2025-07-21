use std::collections::HashMap;

use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub(crate) struct Storage {
    shared: Arc<Shared>,
}

impl Storage {
    pub(crate) fn setup() -> Storage {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                entries: HashMap::new(),
            }),
        });

        Storage { shared }
    }

    pub(crate) async fn get(&self, key: &str) -> Option<Bytes> {
        let state = self.shared.state.lock().await;
        state.entries.get(key).map(|enty| enty.data.clone())
    }

    pub(crate) async fn set(&self, key: &String, val: &Bytes) {
        let mut state = self.shared.state.lock().await;
        state
            .entries
            .insert(key.clone(), Entry { data: val.clone() });
    }
}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    entries: HashMap<String, Entry>,
}

#[derive(Debug)]
struct Entry {
    data: Bytes,
}
