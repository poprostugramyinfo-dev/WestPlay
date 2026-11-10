//! Public HaulMP live-map positions for the in-cab map.
//!
//! The live-map API does not send `Access-Control-Allow-Origin`, so WebView2 cannot read it from the cabin
//! page. This small worker fetches the public feed only while the player has enabled the map layer and passes
//! the positions back to the page through the existing WebView bridge.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const FEED: &str = "https://map.haulmp.com/api/live";
const POLL_EVERY: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, serde::Serialize)]
pub struct Player {
    pub id: u32,
    pub x: f64,
    pub z: f64,
    pub heading: f64,
}

#[derive(serde::Deserialize)]
struct LiveResponse {
    #[serde(default)]
    online: bool,
    #[serde(default)]
    players: Vec<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct LivePlayer {
    id: u32,
    x: f64,
    z: f64,
    h: f64,
}

fn read_players(response: LiveResponse) -> Vec<Player> {
    if !response.online {
        return Vec::new();
    }
    response
        .players
        .into_iter()
        .filter_map(|entry| serde_json::from_value::<LivePlayer>(entry).ok())
        .filter(|player| {
            player.id > 0
                && player.x.is_finite()
                && player.z.is_finite()
                && player.h.is_finite()
                && player.x.abs() <= 500_000.0
                && player.z.abs() <= 500_000.0
        })
        .map(|player| Player { id: player.id, x: player.x, z: player.z, heading: player.h })
        .collect()
}

pub struct Watch {
    enabled: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    updates: Receiver<Vec<Player>>,
    thread: Option<JoinHandle<()>>,
}

impl Watch {
    pub fn start() -> Self {
        let enabled = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_enabled = enabled.clone();
        let worker_stop = stop.clone();
        let (sender, updates) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("westplay-haulmp".into())
            .spawn(move || poll(worker_enabled, worker_stop, sender))
            .ok();
        Self { enabled, stop, updates, thread }
    }

    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::Release);
    }

    /// The newest complete read, if one arrived since the last host tick.
    pub fn take_update(&self) -> Option<Vec<Player>> {
        if !self.enabled.load(Ordering::Acquire) {
            while self.updates.try_recv().is_ok() {}
            return None;
        }
        let mut latest = None;
        while let Ok(players) = self.updates.try_recv() {
            latest = Some(players);
        }
        latest
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn poll(enabled: Arc<AtomicBool>, stop: Arc<AtomicBool>, updates: SyncSender<Vec<Player>>) {
    let Ok(client) = reqwest::blocking::Client::builder().timeout(Duration::from_secs(5)).build() else {
        return;
    };
    while !stop.load(Ordering::Acquire) {
        if !enabled.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(200));
            continue;
        }
        let players = client
            .get(FEED)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .and_then(|response| response.json::<LiveResponse>())
            .map(read_players)
            .unwrap_or_default();
        // A failed or offline read clears old positions instead of leaving stopped trucks on the map.
        let _ = updates.try_send(players);
        let until = std::time::Instant::now() + POLL_EVERY;
        while !stop.load(Ordering::Acquire)
            && enabled.load(Ordering::Acquire)
            && std::time::Instant::now() < until
        {
            thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LiveResponse, read_players};

    #[test]
    fn keeps_only_valid_player_coordinates_and_drops_names() {
        let response: LiveResponse = serde_json::from_str(
            r#"{"online":true,"players":[
                {"id":7,"name":"Driver","x":12.5,"y":4,"z":-80,"h":1.2},
                {"id":8,"x":500001,"z":0,"h":0},
                {"id":0,"x":0,"z":0,"h":0},
                {"id":9,"x":0,"z":0,"h":"north"}
            ]}"#,
        )
        .unwrap();

        let players = read_players(response);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].id, 7);
        assert_eq!((players[0].x, players[0].z, players[0].heading), (12.5, -80.0, 1.2));
    }

    #[test]
    fn clears_players_when_the_live_map_is_offline() {
        let response: LiveResponse = serde_json::from_str(
            r#"{"online":false,"players":[{"id":7,"x":12.5,"z":-80,"h":1.2}]}"#,
        )
        .unwrap();

        assert!(read_players(response).is_empty());
    }
}
