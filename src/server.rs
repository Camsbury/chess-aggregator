use actix_web::{get, web, App, HttpServer, Responder};
use crate::chess_db;
use crate::game_stats::GameStats;
use rocksdb::{Options, DB};
use serde::Deserialize;
use shakmaty::{fen::Fen, CastlingMode, Chess};

#[derive(Deserialize)]
struct Params {
    fen: String,
}

struct AppState {
    db: DB
}

impl AppState {
    fn new(db_path: String) -> AppState {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        let db = DB::open(&db_opts, db_path).unwrap();
        AppState {db}
    }
}

#[get("/")]
async fn index(
    data: web::Data<AppState>,
    params: web::Query<Params>,
) -> impl Responder {
    //TODO: handle errors for all the following unwraps
    let fen: Fen = params.fen.parse().unwrap();
    let pos: Chess = fen.into_position(CastlingMode::Standard).unwrap();
    let stats: GameStats = chess_db::get_pos_stats(&data.db, &pos).unwrap();
    web::Json(stats)
}

#[actix_web::main]
pub async fn serve(
    db_path: String
) -> std::io::Result<()> {
    let server = HttpServer::new(
        move || App::new()
            .app_data(web::Data::new(AppState::new(db_path.clone())))
            .service(index)
    ).bind("127.0.0.1:9090").unwrap();

    server.run().await
}

// stream
//     .for_each_concurrent(None, |stream_item| {
//         let a = a.clone();
//         async move {
//             println!("{}", a);
//             println!("{}", stream_item);
//         }
//     })
//     .await;
