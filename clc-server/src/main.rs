use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use warp::{ws::Message, Filter, Rejection};
use clc_lib::protocol::{ChatId, InviteId, UserId, UserName};

mod handler;
mod ws;

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! debug {
    () => {
        println!()
    };
    ($($arg:tt)*) => {
        println!($($arg)*)
    };
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! debug {
    () => {};
    ($($arg:tt)*) => {};
}

type Result<T> = std::result::Result<T, Rejection>;
type Clients = Arc<RwLock<HashMap<UserId, Client>>>;
type Chats = Arc<RwLock<HashMap<ChatId, Chat>>>;

#[derive(Debug, Clone)]
pub(crate) struct Client {
    pub(crate) user_name: UserName,
    pub(crate) topics: Vec<String>,
    pub(crate) sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Chat {
    pub(crate) users: Vec<UserId>,
    pub(crate) invites: HashMap<InviteId, UserName>,
    pub(crate) sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

#[tokio::main]
async fn main() {
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
    let chats: Chats = Arc::new(RwLock::new(HashMap::new()));

    let health_route = warp::path("api/health").and_then(handler::health);

    let register = warp::path("api/register");
    let register_routes = register
        .and(warp::post())
        .and(warp::body::json())
        .and(with(clients.clone()))
        .and_then(handler::register)
        .or(register
            .and(warp::delete())
            .and(warp::body::json())
            .and(with(clients.clone()))
            .and_then(handler::unregister));

    let publish = warp::path("api/room/create")
        .and(warp::body::json())
        .and(with(clients.clone()))
        .and_then(handler::publish_handler);

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with(clients.clone()))
        .and_then(handler::ws_handler);

    let routes = health_route
        .or(register_routes)
        .or(ws_route)
        .or(publish)
        .with(warp::cors().allow_any_origin());

    warp::serve(routes)
        //.tls()
        //.cert_path("tls/cert.pem")
        //.key_path("tls/key.rsa")
        .run(([127, 0, 0, 1], 8000)).await;
}

fn with<T: Clone + Send>(data: T) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
    warp::any().map(move || data.clone())
}