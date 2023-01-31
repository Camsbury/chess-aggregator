use crate::chess_db;
use crate::game_stats::GameStats;
use actix_web::{get, web, App, HttpServer, Responder};
use rocksdb::{Options, DB};
use serde::Deserialize;
use shakmaty::{fen::Fen, CastlingMode, Chess};

#[derive(Deserialize)]
struct Params {
    fen: String,
}

struct AppState {
    db_path: String,
}

#[get("/")]
async fn index(
    data: web::Data<AppState>,
    params: web::Query<Params>,
) -> impl Responder {
    //TODO: handle errors for all the following unwraps
    let mut db_opts = Options::default();
    db_opts.create_if_missing(true);
    let db = DB::open(&db_opts, data.db_path.clone()).unwrap();
    let fen: Fen = params.fen.parse().unwrap();
    let pos: Chess = fen.into_position(CastlingMode::Standard).unwrap();
    let stats: GameStats = chess_db::get_pos_stats(&db, &pos).unwrap();
    web::Json(stats)
}

#[actix_web::main]
pub async fn serve(db_path: String) -> std::io::Result<()> {
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                db_path: db_path.clone(),
            }))
            .service(index)
    })
    .bind("127.0.0.1:9090")
    .unwrap();

    server.run().await
}
