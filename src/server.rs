use actix_web::{
    get,
    web,
    error::ErrorInternalServerError,
    App,
    HttpServer,
    Result,
};
use crate::chess_db::ChessDB;
use crate::config;
use crate::merge::wins_merge_op;
use crate::rocks_cfg;
use crate::{MoveResult, PositionResult};
use rocksdb::DB;
use serde::Deserialize;
use shakmaty::{uci::Uci, san::SanPlus, fen::Fen, CastlingMode, Chess};

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
) -> Result<web::Json<PositionResult>> {
    let mut db_opts = rocks_cfg::tuned();
    db_opts.set_merge_operator_associative("add_wins", wins_merge_op);
    let db = DB::open(&db_opts, data.db_path.clone())
        .expect("Failed to open the database!");
    let fen: Fen = params.fen.parse().expect("invalid FEN!");
    let pos: Chess = fen
        .into_position(CastlingMode::Standard)
        .expect("Not a parseable FEN?!");
    let mut cdb = ChessDB::new(&db);
    let stats = cdb.get_pos_stats(&pos)
        .expect("Failed getting position stats");

    // --- convert the HashMap<String, GameWins> into a Vec<MoveResult> ---
    let mut moves = Vec::with_capacity(stats.game_moves.len());
    for (uci_str, wins) in stats.game_moves {
        // ① parse the UCI, ② turn it into a Move, ③ render SAN
        let mv  = Uci::from_ascii(uci_str.as_bytes())
            .map_err(ErrorInternalServerError)? // ➍ UCI was malformed
            .to_move(&pos)
            .map_err(ErrorInternalServerError)?;

        let san = SanPlus::from_move(pos.clone(), &mv).to_string();

        moves.push(MoveResult {
            uci:   uci_str,
            san,
            white: wins.white,
            black: wins.black,
            draws: wins.draws,                      // field is named `draw` in GameWins:contentReference[oaicite:1]{index=1}
        });
    }

    // --- assemble the final JSON object ---
    let body = PositionResult {
        white: stats.game_wins.white,
        black: stats.game_wins.black,
        draws: stats.game_wins.draws,               // same rename here
        moves,
    };

    Ok(web::Json(body))
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
