use serde_derive::Serialize;
use warp::http::StatusCode;
use warp::{Filter, Reply};

#[derive(Serialize)]
struct NoteResponse {
    title: String,
}

#[derive(Serialize)]
struct NotesResponse(Vec<NoteResponse>);

#[tokio::main]
async fn main() {
    let get_notes_handler = warp::path!("notes").and(warp::get()).map(move || {
        warp::reply::json(&NotesResponse(vec![NoteResponse {
            title: "Lorem Ipsum".to_string(),
        }]))
    });

    let post_note_handler = warp::path!("notes").and(warp::post()).map(move || {
        warp::reply::json(&NoteResponse {
            title: "Lorem Ipsum".to_string(),
        })
    });

    let not_found_handler = warp::any().map(move || StatusCode::NOT_FOUND.into_response());

    let routes = get_notes_handler
        .or(post_note_handler)
        .or(not_found_handler);

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
