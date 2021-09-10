use std::convert::Infallible;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use serde_derive::{Deserialize, Serialize};
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::{Filter, Reply};

#[derive(Serialize)]
struct NoteResponse {
    title: String,
}

#[derive(Serialize)]
struct NotesResponse(Vec<NoteResponse>);

#[derive(Deserialize)]
struct CreateNoteRequest {
    title: String,
}

#[derive(Clone)]
struct Note {
    id: u64,
    title: String,
}

type NoteDatabase = Arc<Mutex<Vec<Note>>>;

static NEXT_NOTE_ID: AtomicU64 = AtomicU64::new(0);

async fn list_notes(db: NoteDatabase) -> Result<impl Reply, Infallible> {
    let db = db.lock().await;

    let notes = db
        .iter()
        .map(|note| NoteResponse {
            title: note.title.to_owned(),
        })
        .collect();

    Ok(warp::reply::json(&NotesResponse(notes)))
}

async fn create_note(db: NoteDatabase, req: CreateNoteRequest) -> Result<impl Reply, Infallible> {
    let new_note = Note {
        id: NEXT_NOTE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        title: req.title,
    };

    let mut db = db.lock().await;
    db.push(new_note.clone());

    Ok(warp::reply::json(&NoteResponse {
        title: new_note.title,
    }))
}

#[tokio::main]
async fn main() {
    let db = NoteDatabase::new(Mutex::new(Vec::new()));

    let note_database_filter = warp::any().map(move || db.clone());

    let list_notes_handler = warp::path!("notes")
        .and(warp::get())
        .and(note_database_filter.clone())
        .and_then(list_notes);

    let post_note_handler = warp::path!("notes")
        .and(warp::post())
        .and(note_database_filter)
        .and(warp::body::content_length_limit(1024 * 16).and(warp::body::json()))
        .and_then(create_note);

    let not_found_handler = warp::any().map(move || StatusCode::NOT_FOUND.into_response());

    let routes = list_notes_handler
        .or(post_note_handler)
        .or(not_found_handler);

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
