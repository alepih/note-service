use std::convert::Infallible;
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use serde_derive::{Deserialize, Serialize};
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::{Filter, Reply};

#[derive(Clone, Deserialize, Serialize, PartialEq)]
struct NoteId(u64);

impl FromStr for NoteId {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(id) = u64::from_str(s) {
            Ok(NoteId(id))
        } else {
            Err(())
        }
    }
}

#[derive(Serialize)]
struct NoteResponse {
    id: NoteId,
    title: String,
    content: String,
}

#[derive(Serialize)]
struct NotesResponse(Vec<NoteResponse>);

#[derive(Deserialize)]
struct CreateNoteRequest {
    title: String,
    #[serde(default)]
    content: String,
}

#[derive(Deserialize)]
struct UpdateNoteRequest {
    title: Option<String>,
    content: Option<String>,
}

#[derive(Clone)]
struct Note {
    id: NoteId,
    title: String,
    content: String,
}

type NoteDatabase = Arc<Mutex<Vec<Note>>>;

static NEXT_NOTE_ID: AtomicU64 = AtomicU64::new(1);

async fn list_notes(db: NoteDatabase) -> Result<impl Reply, Infallible> {
    let db = db.lock().await;

    let notes = db
        .iter()
        .map(|note| NoteResponse {
            id: note.id.clone(),
            title: note.title.to_owned(),
            content: note.content.to_owned(),
        })
        .collect();

    Ok(warp::reply::json(&NotesResponse(notes)))
}

async fn create_note(db: NoteDatabase, req: CreateNoteRequest) -> Result<impl Reply, Infallible> {
    let new_note = Note {
        id: NoteId(NEXT_NOTE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)),
        title: req.title,
        content: req.content,
    };

    let mut db = db.lock().await;
    db.push(new_note.clone());

    let body = warp::reply::json(&NoteResponse {
        id: new_note.id,
        title: new_note.title,
        content: new_note.content,
    });
    Ok(warp::reply::with_status(body, StatusCode::CREATED))
}

async fn update_note(id: NoteId, db: NoteDatabase, req: UpdateNoteRequest) -> Result<impl Reply, Infallible> {
    if req.title.is_none() && req.content.is_none() {
        return Ok(StatusCode::UNPROCESSABLE_ENTITY.into_response());
    }

    let mut db = db.lock().await;
    let note_idx = db.iter().position(|note| note.id == id);
    let note = match note_idx.and_then(|idx| db.get_mut(idx)) {
        Some(note) => note,
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
    };

    if let Some(new_title) = req.title {
        note.title = new_title;
    }
    if let Some(new_content) = req.content {
        note.content = new_content;
    }
    
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn remove_note(id: NoteId, db: NoteDatabase) -> Result<impl Reply, Infallible> {
    let mut db = db.lock().await;
    let old_len = db.len();
    db.retain(|note| note.id != id);

    if old_len != db.len() {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Ok(StatusCode::NOT_FOUND.into_response())
    }
}

#[tokio::main]
async fn main() {
    let db = NoteDatabase::new(Mutex::new(Vec::new()));

    let note_database_filter = warp::any().map(move || db.clone());

    let list_notes_handler = warp::path!("notes")
        .and(warp::get())
        .and(note_database_filter.clone())
        .and_then(list_notes);

    let create_note_handler = warp::path!("notes")
        .and(warp::post())
        .and(note_database_filter.clone())
        .and(warp::body::content_length_limit(1024 * 16).and(warp::body::json()))
        .and_then(create_note);

    let update_note_handler = warp::path!("notes" / NoteId)
        .and(warp::patch())
        .and(note_database_filter.clone())
        .and(warp::body::content_length_limit(1024 * 16).and(warp::body::json()))
        .and_then(update_note);

    let remove_note_handler = warp::path!("notes" / NoteId)
        .and(warp::delete())
        .and(note_database_filter)
        .and_then(remove_note);

    let not_found_handler = warp::any().map(move || StatusCode::NOT_FOUND.into_response());

    let routes = list_notes_handler
        .or(create_note_handler)
        .or(update_note_handler)
        .or(remove_note_handler)
        .or(not_found_handler);

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
