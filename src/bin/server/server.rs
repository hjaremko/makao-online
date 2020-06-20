use tokio::prelude::*;
use tokio::net::TcpStream;
use log::{error, debug};
use std::str;
use std::net::{Shutdown};
use crate::server::ServerState::{Idle, Playing};

#[derive(Clone, Copy)]
pub enum ServerState {
    Playing,
    Idle,
}

#[derive(Copy, Clone)]
pub struct GameServer {
    current: i32,
    state: ServerState,
}

impl GameServer {
    pub fn new() -> Self {
        GameServer { current: 0, state: Idle }
    }

    pub fn current_players(&self) -> i32 {
        self.current
    }

    pub fn game_state(&self) -> ServerState {
        self.state.clone()
    }

    pub fn start_game(&mut self) {
        debug!("starting game");
        self.state = Playing;
    }

    pub fn inc(&mut self) {
        self.current += 1;
    }

    pub async fn handle_req(& self, request: &str) {
        debug!("handling: {}", request);
    }
}
