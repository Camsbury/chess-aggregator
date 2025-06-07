use actix_web::{get, web, App, HttpServer, Responder};
use crate::chess_db::ChessDB;
use crate::config;
use crate::game_stats::GameStats;
use crate::merge::wins_merge_op;
use crate::rocks_cfg;
use rocksdb::DB;
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
    let mut db_opts = rocks_cfg::tuned();
    db_opts.set_merge_operator_associative("add_wins", wins_merge_op);
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
pub async fn serve(cfg: config::Server) -> std::io::Result<()> {
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                db_path: cfg.db_path.clone(),
            }))
            .service(index)
    })
    .bind("127.0.0.1:9090")
    .unwrap();

    server.run().await
}
