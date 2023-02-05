use crate::chess_db::ChessDB;
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
    let db = DB::open(&db_opts, data.db_path.clone())
        .expect("Failed to open the database!");
    let fen: Fen = params.fen.parse().expect("invalid FEN!");
    let pos: Chess = fen
        .into_position(CastlingMode::Standard)
        .expect("Not a parseable FEN?!");
    let mut cdb = ChessDB::new(&db);
    let stats: GameStats = cdb
        .get_pos_stats(&pos)
        .expect("Failed getting position stats");
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
