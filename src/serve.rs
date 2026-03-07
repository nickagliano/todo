use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use tower_http::trace::TraceLayer;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, patch, put},
};
use serde::{Deserialize, Serialize};

use crate::{
    reading::{JsonReadingList, ReadingItem, ReadingList},
    storage::{JsonStorage, Storage},
    task::Task,
};

type SharedStorage = Arc<Mutex<JsonStorage>>;
type SharedReadingList = Arc<Mutex<JsonReadingList>>;
type SharedHouseStorage = Arc<Mutex<JsonStorage>>;

#[derive(Clone)]
struct AppState {
    storage: SharedStorage,
    reading: SharedReadingList,
    house: SharedHouseStorage,
}

pub async fn run(port: u16) -> Result<()> {
    let storage = Arc::new(Mutex::new(JsonStorage::default()));
    let reading = Arc::new(Mutex::new(JsonReadingList::default()));
    let house_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("simple_todo")
        .join("house_projects.json");
    let house = Arc::new(Mutex::new(JsonStorage::new(house_path)));

    let app = build_router(storage, reading, house);

    tracing_subscriber::fmt::init();

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("simple_todo serving on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn build_router(storage: SharedStorage, reading: SharedReadingList, house: SharedHouseStorage) -> Router {
    let state = AppState { storage, reading, house };
    Router::new()
        .route("/", get(index))
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/reorder", put(reorder_tasks))
        .route("/tasks/{id}", patch(update_task).delete(delete_task))
        .route("/reading", get(list_reading).post(create_reading))
        .route("/reading/{id}", patch(update_reading).delete(delete_reading))
        .route("/reading/{id}/view", get(view_reading))
        .route("/house", get(house_index))
        .route("/house-projects", get(list_house).post(create_house))
        .route("/house-projects/reorder", put(reorder_house))
        .route("/house-projects/{id}", patch(update_house).delete(delete_house))
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

// ── HTML helpers ──────────────────────────────────────────────────────────────

fn html_response(body: impl Into<String>) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/html; charset=utf-8".parse().unwrap());
    (headers, body.into())
}

// ── GET / ─────────────────────────────────────────────────────────────────────

async fn index() -> impl IntoResponse {
    let html = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>simple_todo</title>
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: #f5f5f5;
      color: #1a1a1a;
      min-height: 100vh;
      display: flex;
      justify-content: center;
      padding: 3rem 1rem;
    }
    main {
      width: 100%;
      max-width: 540px;
    }
    h1 {
      font-size: 1.5rem;
      font-weight: 600;
      margin-bottom: 1.5rem;
      letter-spacing: -0.02em;
    }
    .tabs {
      display: flex;
      gap: 0.25rem;
      margin-bottom: 1.5rem;
      border-bottom: 1px solid #e0e0e0;
    }
    .tab-btn {
      padding: 0.5rem 1rem;
      background: none;
      border: none;
      border-bottom: 2px solid transparent;
      font-size: 0.95rem;
      cursor: pointer;
      color: #666;
      margin-bottom: -1px;
      text-decoration: none;
      display: inline-block;
    }
    .tab-btn.active {
      color: #1a1a1a;
      border-bottom-color: #1a1a1a;
      font-weight: 500;
    }
    .tab-panel { display: none; }
    .tab-panel.active { display: block; }
    form {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 1.5rem;
    }
    input[type="text"] {
      flex: 1;
      padding: 0.6rem 0.75rem;
      border: 1px solid #d1d1d1;
      border-radius: 6px;
      font-size: 0.95rem;
      outline: none;
      background: #fff;
    }
    input[type="text"]:focus { border-color: #555; }
    button.add {
      padding: 0.6rem 1rem;
      background: #1a1a1a;
      color: #fff;
      border: none;
      border-radius: 6px;
      font-size: 0.95rem;
      cursor: pointer;
    }
    button.add:hover { background: #333; }
    ul {
      list-style: none;
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }
    li { position: relative; overflow: hidden; border: 1px solid #e8e8e8; border-radius: 8px; }
    .item-row {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      background: #fff;
      padding: 0.7rem 0.85rem;
      border-radius: 8px;
      position: relative;
      z-index: 1;
    }
    li.done .item-row span { text-decoration: line-through; color: #999; }
    li.read .item-title { color: #999; }
    li.read .item-domain { color: #bbb; }
    input[type="checkbox"] { width: 1.1rem; height: 1.1rem; cursor: pointer; flex-shrink: 0; }
    li span { flex: 1; font-size: 0.95rem; line-height: 1.4; }
    .item-info { flex: 1; display: flex; flex-direction: column; gap: 0.15rem; min-width: 0; }
    .item-title { font-size: 0.95rem; color: #1a1a1a; text-decoration: none; }
    .item-title:hover { text-decoration: underline; }
    .item-domain { font-size: 0.8rem; color: #888; }
    button.del { background: none; border: none; color: #ccc; font-size: 1.1rem; cursor: pointer; line-height: 1; padding: 0 0.1rem; flex-shrink: 0; }
    button.del:hover { color: #e55; }
    .swipe-delete {
      position: absolute; right: 0; top: 0; bottom: 0; width: 80px;
      background: #e55; color: #fff; border: none; cursor: pointer;
      font-size: 0.875rem; font-weight: 500;
      display: flex; align-items: center; justify-content: center;
      border-radius: 0 8px 8px 0;
    }
    p.empty { color: #999; font-size: 0.9rem; }
    .drag-handle { color: #ccc; cursor: grab; font-size: 1.1rem; padding: 0 0 0 0.15rem; flex: none; touch-action: none; user-select: none; line-height: 1; }
    .drag-handle:active { cursor: grabbing; }
    li.dragging { opacity: 0.3; }
  </style>
</head>
<body>
  <main>
    <h1>simple_todo</h1>

    <div class="tabs">
      <button class="tab-btn active" onclick="switchTab('tasks', this)">Tasks</button>
      <button class="tab-btn" onclick="switchTab('reading', this)">Reading List</button>
      <a href="/house" class="tab-btn">House Projects</a>
    </div>

    <div id="tab-tasks" class="tab-panel active">
      <form id="add-form">
        <input type="text" id="text-input" placeholder="Add a task…" autocomplete="off">
        <button type="submit" class="add">Add</button>
      </form>
      <ul id="list"></ul>
      <p class="empty" id="empty" hidden>No tasks yet.</p>
    </div>

    <div id="tab-reading" class="tab-panel">
      <form id="reading-form">
        <input type="text" id="url-input" placeholder="Add a URL…" autocomplete="off">
        <button type="submit" class="add">Add</button>
      </form>
      <ul id="reading-list"></ul>
      <p class="empty" id="reading-empty" hidden>No items yet.</p>
    </div>
  </main>

  <script>
    function switchTab(name, btn) {
      document.querySelectorAll('.tab-panel').forEach(p => p.classList.remove('active'));
      document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
      document.getElementById('tab-' + name).classList.add('active');
      btn.classList.add('active');
    }

    // Support ?tab=reading so the Reading List link from /house works
    if (new URLSearchParams(window.location.search).get('tab') === 'reading') {
      const readingBtn = document.querySelector('.tab-btn:nth-child(2)');
      switchTab('reading', readingBtn);
    }

    // ── Tasks ──────────────────────────────────────────────────────────────────
    const list  = document.getElementById('list');
    const empty = document.getElementById('empty');
    const form  = document.getElementById('add-form');
    const input = document.getElementById('text-input');

    async function loadTasks() {
      const tasks = await fetch('/tasks').then(r => r.json());
      renderTasks(tasks);
    }

    function renderTasks(tasks) {
      list.innerHTML = '';
      empty.hidden = tasks.length > 0;
      tasks.forEach(t => {
        const li = document.createElement('li');
        li.dataset.id = t.id;
        if (t.done) li.classList.add('done');

        const swipeDel = document.createElement('button');
        swipeDel.className = 'swipe-delete';
        swipeDel.textContent = 'Delete';

        const row = document.createElement('div');
        row.className = 'item-row';

        const cb = document.createElement('input');
        cb.type = 'checkbox';
        cb.checked = t.done;
        cb.onchange = async () => {
          await fetch(`/tasks/${t.id}`, {
            method: 'PATCH',
            headers: {'content-type': 'application/json'},
            body: JSON.stringify({ done: cb.checked }),
          });
          loadTasks();
        };

        const span = document.createElement('span');
        span.textContent = t.text;

        const handle = document.createElement('span');
        handle.className = 'drag-handle';
        handle.textContent = '⠿';

        row.append(cb, span, handle);
        li.append(swipeDel, row);
        list.appendChild(li);
      });
    }

    form.onsubmit = async e => {
      e.preventDefault();
      const text = input.value.trim();
      if (!text) return;
      await fetch('/tasks', {
        method: 'POST',
        headers: {'content-type': 'application/json'},
        body: JSON.stringify({ text }),
      });
      input.value = '';
      loadTasks();
    };

    // ── Reading List ───────────────────────────────────────────────────────────
    const readingList  = document.getElementById('reading-list');
    const readingEmpty = document.getElementById('reading-empty');
    const readingForm  = document.getElementById('reading-form');
    const urlInput     = document.getElementById('url-input');

    async function loadReading() {
      const items = await fetch('/reading').then(r => r.json());
      renderReading(items);
    }

    function renderReading(items) {
      readingList.innerHTML = '';
      readingEmpty.hidden = items.length > 0;
      items.forEach(item => {
        const li = document.createElement('li');
        if (item.read) li.classList.add('read');

        const row = document.createElement('div');
        row.className = 'item-row';

        const cb = document.createElement('input');
        cb.type = 'checkbox';
        cb.checked = item.read;
        cb.title = 'Mark as read';
        cb.onchange = async () => {
          await fetch(`/reading/${item.id}`, {
            method: 'PATCH',
            headers: {'content-type': 'application/json'},
            body: JSON.stringify({ read: cb.checked }),
          });
          loadReading();
        };

        let domain = '';
        try { domain = new URL(item.url).hostname; } catch (_) {}

        const info = document.createElement('div');
        info.className = 'item-info';

        const title = document.createElement('a');
        title.className = 'item-title';
        title.href = `/reading/${item.id}/view`;
        title.textContent = item.title;

        const domainEl = document.createElement('span');
        domainEl.className = 'item-domain';
        domainEl.textContent = domain;

        info.append(title, domainEl);

        const del = document.createElement('button');
        del.className = 'del';
        del.textContent = '×';
        del.onclick = async () => {
          await fetch(`/reading/${item.id}`, { method: 'DELETE' });
          loadReading();
        };

        row.append(cb, info, del);
        li.append(row);
        readingList.appendChild(li);
      });
    }

    readingForm.onsubmit = async e => {
      e.preventDefault();
      const url = urlInput.value.trim();
      if (!url) return;
      await fetch('/reading', {
        method: 'POST',
        headers: {'content-type': 'application/json'},
        body: JSON.stringify({ url }),
      });
      urlInput.value = '';
      loadReading();
    };

    // ── Drag-to-reorder ────────────────────────────────────────────────────────
    function makeSortable(ul, endpoint) {
      let dragging = null, ghost = null, lastY = 0;

      function moveDrag(y) {
        if (!dragging) return;
        ghost.style.top = (parseFloat(ghost.style.top) + y - lastY) + 'px';
        lastY = y;
        const siblings = [...ul.querySelectorAll('li:not(.dragging)')];
        let target = null;
        for (const s of siblings) {
          const r = s.getBoundingClientRect();
          if (y < r.top + r.height / 2) { target = s; break; }
        }
        if (target) ul.insertBefore(dragging, target);
        else ul.appendChild(dragging);
      }

      function endDrag() {
        if (!dragging) return;
        dragging.classList.remove('dragging');
        ghost.remove(); ghost = null;
        const ids = [...ul.querySelectorAll('li')].map(li => parseInt(li.dataset.id));
        fetch(endpoint, { method: 'PUT', headers: {'content-type':'application/json'}, body: JSON.stringify({ids}) });
        document.removeEventListener('mousemove', onMM);
        document.removeEventListener('mouseup', endDrag);
        document.removeEventListener('touchmove', onTM);
        document.removeEventListener('touchend', endDrag);
        dragging = null;
      }

      function onMM(e) { moveDrag(e.clientY); }
      function onTM(e) { e.preventDefault(); moveDrag(e.touches[0].clientY); }

      function startDrag(li, y) {
        // Close any open swipe items first
        ul.querySelectorAll('li[data-open="1"]').forEach(item => {
          const r = item.querySelector('.item-row');
          if (r) { r.style.transition = ''; r.style.transform = ''; }
          item.dataset.open = '';
        });
        dragging = li; lastY = y;
        const r = li.getBoundingClientRect();
        ghost = li.cloneNode(true);
        const ghostDel = ghost.querySelector('.swipe-delete');
        if (ghostDel) ghostDel.style.display = 'none';
        ghost.style.cssText = `position:fixed;top:${r.top}px;left:${r.left}px;width:${r.width}px;opacity:0.7;pointer-events:none;z-index:9999;box-shadow:0 4px 12px rgba(0,0,0,0.15);border-radius:8px;`;
        document.body.appendChild(ghost);
        li.classList.add('dragging');
      }

      ul.addEventListener('mousedown', e => {
        if (!e.target.closest('.drag-handle')) return;
        e.preventDefault();
        startDrag(e.target.closest('li'), e.clientY);
        document.addEventListener('mousemove', onMM);
        document.addEventListener('mouseup', endDrag);
      });

      ul.addEventListener('touchstart', e => {
        if (!e.target.closest('.drag-handle')) return;
        e.preventDefault();
        startDrag(e.target.closest('li'), e.touches[0].clientY);
        document.addEventListener('touchmove', onTM, {passive: false});
        document.addEventListener('touchend', endDrag);
      }, {passive: false});
    }

    function makeSwipeable(ul, deleteUrl, reload) {
      let target = null, startX = 0, startY = 0, dirLocked = false;
      const W = 80;

      function getRow(li) { return li.querySelector('.item-row'); }
      function setX(li, x) {
        const row = getRow(li);
        if (row) row.style.transform = x ? `translateX(${-x}px)` : '';
      }
      function snap(li, open) {
        const row = getRow(li);
        if (!row) return;
        row.style.transition = 'transform 0.2s ease';
        setX(li, open ? W : 0);
        li.dataset.open = open ? '1' : '';
        setTimeout(() => { row.style.transition = ''; }, 200);
      }
      function closeAll(except) {
        ul.querySelectorAll('li[data-open="1"]').forEach(li => {
          if (li !== except) snap(li, false);
        });
      }

      ul.addEventListener('touchstart', e => {
        const li = e.target.closest('li');
        if (!li || e.target.closest('.drag-handle')) return;
        target = li; startX = e.touches[0].clientX; startY = e.touches[0].clientY; dirLocked = false;
        closeAll(li);
      }, {passive: true});

      ul.addEventListener('touchmove', e => {
        if (!target) return;
        const dx = e.touches[0].clientX - startX;
        const dy = e.touches[0].clientY - startY;
        if (!dirLocked) {
          if (Math.abs(dy) > Math.abs(dx) + 3) { target = null; return; }
          if (Math.abs(dx) > 5) dirLocked = true; else return;
        }
        const base = target.dataset.open === '1' ? W : 0;
        setX(target, Math.max(0, Math.min(W, base - dx)));
      }, {passive: true});

      ul.addEventListener('touchend', e => {
        if (!target) return;
        const dx = e.changedTouches[0].clientX - startX;
        snap(target, target.dataset.open === '1' ? dx > -(W / 2) : -dx > W / 2);
        target = null;
      });

      ul.addEventListener('click', async e => {
        const btn = e.target.closest('.swipe-delete');
        if (btn) {
          const li = btn.closest('li');
          await fetch(`${deleteUrl}/${li.dataset.id}`, { method: 'DELETE' });
          reload();
        } else {
          closeAll(null);
        }
      });

      document.addEventListener('touchstart', e => {
        if (!ul.contains(e.target)) closeAll(null);
      }, {passive: true});
    }

    loadTasks();
    loadReading();
    makeSortable(list, '/tasks/reorder');
    makeSwipeable(list, '/tasks', loadTasks);
  </script>
</body>
</html>"#;

    html_response(html)
}

// ── Task handlers ─────────────────────────────────────────────────────────────

async fn list_tasks(State(state): State<AppState>) -> impl IntoResponse {
    let tasks = state.storage.lock().unwrap().load().unwrap_or_default();
    Json(tasks)
}

#[derive(Deserialize)]
struct CreateTaskBody {
    text: String,
}

async fn create_task(
    State(state): State<AppState>,
    Json(body): Json<CreateTaskBody>,
) -> impl IntoResponse {
    let store = state.storage.lock().unwrap();
    let mut tasks = store.load().unwrap_or_default();
    let id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let task = Task::new(id, body.text);
    tasks.push(task.clone());
    store.save(&tasks).unwrap();
    (StatusCode::CREATED, Json(task))
}

#[derive(Deserialize)]
struct UpdateTaskBody {
    done: bool,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<u32>,
    Json(body): Json<UpdateTaskBody>,
) -> impl IntoResponse {
    let store = state.storage.lock().unwrap();
    let mut tasks = store.load().unwrap_or_default();
    match tasks.iter_mut().find(|t| t.id == id) {
        Some(t) => {
            t.done = body.done;
            let updated = t.clone();
            store.save(&tasks).unwrap();
            (StatusCode::OK, Json(updated)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody { error: format!("no task #{id}") }),
        )
            .into_response(),
    }
}

async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let store = state.storage.lock().unwrap();
    let mut tasks = store.load().unwrap_or_default();
    match tasks.iter().position(|t| t.id == id) {
        Some(pos) => {
            tasks.remove(pos);
            store.save(&tasks).unwrap();
            StatusCode::NO_CONTENT.into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody { error: format!("no task #{id}") }),
        )
            .into_response(),
    }
}

// ── Reading list handlers ─────────────────────────────────────────────────────

async fn list_reading(State(state): State<AppState>) -> impl IntoResponse {
    let items = state.reading.lock().unwrap().load().unwrap_or_default();
    Json(items)
}

#[derive(Deserialize)]
struct CreateReadingBody {
    url: String,
}

/// Fetch a URL and extract its <title>. Falls back to the hostname on any error.
async fn fetch_title(url: String) -> String {
    let fallback = hostname_from_url(&url);
    match reqwest::get(url).await {
        Ok(resp) => match resp.text().await {
            Ok(html) => extract_title(&html).unwrap_or(fallback),
            Err(_) => fallback,
        },
        Err(_) => fallback,
    }
}

fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title")?;
    let open_end = lower[start..].find('>')?;
    let content_start = start + open_end + 1;
    let end = lower[content_start..].find("</title>")?;
    let title = html[content_start..content_start + end].trim().to_string();
    if title.is_empty() { None } else { Some(title) }
}

fn hostname_from_url(url: &str) -> String {
    url.splitn(3, '/').nth(2)
        .and_then(|host_and_path| host_and_path.split('/').next())
        .unwrap_or(url)
        .to_string()
}

async fn create_reading(
    State(state): State<AppState>,
    Json(body): Json<CreateReadingBody>,
) -> impl IntoResponse {
    let title = fetch_title(body.url.clone()).await;
    let rl = state.reading.lock().unwrap();
    let mut items = rl.load().unwrap_or_default();
    let id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
    let item = ReadingItem { id, url: body.url, title, read: false };
    items.push(item.clone());
    rl.save(&items).unwrap();
    (StatusCode::CREATED, Json(item))
}

#[derive(Deserialize)]
struct UpdateReadingBody {
    read: bool,
}

async fn update_reading(
    State(state): State<AppState>,
    Path(id): Path<u32>,
    Json(body): Json<UpdateReadingBody>,
) -> impl IntoResponse {
    let rl = state.reading.lock().unwrap();
    let mut items = rl.load().unwrap_or_default();
    match items.iter_mut().find(|i| i.id == id) {
        Some(item) => {
            item.read = body.read;
            let updated = item.clone();
            rl.save(&items).unwrap();
            (StatusCode::OK, Json(updated)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody { error: format!("no reading item #{id}") }),
        )
            .into_response(),
    }
}

async fn delete_reading(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let rl = state.reading.lock().unwrap();
    let mut items = rl.load().unwrap_or_default();
    match items.iter().position(|i| i.id == id) {
        Some(pos) => {
            items.remove(pos);
            rl.save(&items).unwrap();
            StatusCode::NO_CONTENT.into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody { error: format!("no reading item #{id}") }),
        )
            .into_response(),
    }
}

/// Strip all <script> and <style> tags (and their content) from HTML.
fn strip_scripts_styles(html: &str) -> String {
    let mut result = html.to_string();
    for tag in &["script", "style"] {
        loop {
            let lower = result.to_lowercase();
            let open = format!("<{tag}");
            let close = format!("</{tag}>");
            match (lower.find(&open), lower.find(&close)) {
                (Some(start), Some(end)) => {
                    let end_of_close = end + close.len();
                    result = format!("{}{}", &result[..start], &result[end_of_close..]);
                }
                _ => break,
            }
        }
    }
    result
}

/// Find the first <article> or <main> block; fall back to <body>.
fn extract_content(html: &str) -> String {
    let lower = html.to_lowercase();
    for tag in &["article", "main", "body"] {
        let open = format!("<{tag}");
        let close = format!("</{tag}>");
        if let (Some(start), Some(end)) = (lower.find(&open), lower.rfind(&close)) {
            if let Some(open_end) = lower[start..].find('>') {
                let content_start = start + open_end + 1;
                if content_start < end {
                    return html[content_start..end].to_string();
                }
            }
        }
    }
    html.to_string()
}

async fn view_reading(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Response {
    let items = {
        let rl = state.reading.lock().unwrap();
        rl.load().unwrap_or_default()
    };

    let item = match items.iter().find(|i| i.id == id) {
        Some(i) => i.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorBody { error: format!("no reading item #{id}") }),
            )
                .into_response();
        }
    };

    let domain = hostname_from_url(&item.url);
    let raw_html = match reqwest::get(item.url).await {
        Ok(resp) => resp.text().await.unwrap_or_default(),
        Err(_) => String::new(),
    };
    let clean = strip_scripts_styles(&raw_html);
    let content = extract_content(&clean);

    let page = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <style>
    *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Georgia, serif;
      background: #fafaf8;
      color: #1a1a1a;
      padding: 3rem 1rem;
      line-height: 1.7;
    }}
    .reader {{
      max-width: 680px;
      margin: 0 auto;
    }}
    .reader-header {{
      margin-bottom: 2rem;
      padding-bottom: 1rem;
      border-bottom: 1px solid #e0e0e0;
    }}
    .reader-header a {{
      font-size: 0.85rem;
      color: #666;
      text-decoration: none;
    }}
    .reader-header a:hover {{ text-decoration: underline; }}
    .reader-header h1 {{
      font-size: 1.6rem;
      font-weight: 700;
      letter-spacing: -0.02em;
      margin: 0.75rem 0 0.25rem;
      line-height: 1.3;
    }}
    .reader-header .domain {{
      font-size: 0.85rem;
      color: #888;
    }}
    .reader-content img {{ max-width: 100%; height: auto; border-radius: 4px; }}
    .reader-content a {{ color: #1a1a1a; }}
    .reader-content p,
    .reader-content li {{ margin-bottom: 1rem; }}
    .reader-content h1, .reader-content h2, .reader-content h3 {{
      margin: 1.5rem 0 0.5rem;
      line-height: 1.3;
    }}
    .reader-content pre, .reader-content code {{
      font-family: "SF Mono", Menlo, monospace;
      font-size: 0.9em;
      background: #f0f0ec;
      border-radius: 3px;
    }}
    .reader-content pre {{ padding: 1rem; overflow-x: auto; }}
    .reader-content code {{ padding: 0.1em 0.3em; }}
  </style>
</head>
<body>
  <div class="reader">
    <div class="reader-header">
      <a href="/">&larr; Back</a>
      <h1>{title}</h1>
      <span class="domain">{domain}</span>
    </div>
    <div class="reader-content">
      {content}
    </div>
  </div>
</body>
</html>"#,
        title = html_escape(&item.title),
        domain = html_escape(&domain),
        content = content,
    );

    html_response(page).into_response()
}

// ── House Projects page ───────────────────────────────────────────────────────

async fn house_index() -> impl IntoResponse {
    let html = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>House Projects</title>
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: #f5f5f5;
      color: #1a1a1a;
      min-height: 100vh;
      display: flex;
      justify-content: center;
      padding: 3rem 1rem;
    }
    main { width: 100%; max-width: 540px; }
    h1 { font-size: 1.5rem; font-weight: 600; margin-bottom: 1.5rem; letter-spacing: -0.02em; }
    .tabs {
      display: flex;
      gap: 0.25rem;
      margin-bottom: 1.5rem;
      border-bottom: 1px solid #e0e0e0;
    }
    .tab-btn {
      padding: 0.5rem 1rem;
      background: none;
      border: none;
      border-bottom: 2px solid transparent;
      font-size: 0.95rem;
      cursor: pointer;
      color: #666;
      margin-bottom: -1px;
      text-decoration: none;
      display: inline-block;
    }
    .tab-btn.active { color: #1a1a1a; border-bottom-color: #1a1a1a; font-weight: 500; }
    form { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; }
    input[type="text"] {
      flex: 1;
      padding: 0.6rem 0.75rem;
      border: 1px solid #d1d1d1;
      border-radius: 6px;
      font-size: 0.95rem;
      outline: none;
      background: #fff;
    }
    input[type="text"]:focus { border-color: #555; }
    button.add {
      padding: 0.6rem 1rem;
      background: #1a1a1a;
      color: #fff;
      border: none;
      border-radius: 6px;
      font-size: 0.95rem;
      cursor: pointer;
    }
    button.add:hover { background: #333; }
    ul { list-style: none; display: flex; flex-direction: column; gap: 0.5rem; }
    li { position: relative; overflow: hidden; border: 1px solid #e8e8e8; border-radius: 8px; }
    .item-row { display: flex; align-items: center; gap: 0.75rem; background: #fff; padding: 0.7rem 0.85rem; border-radius: 8px; position: relative; z-index: 1; }
    li.done .item-row span { text-decoration: line-through; color: #999; }
    input[type="checkbox"] { width: 1.1rem; height: 1.1rem; cursor: pointer; flex-shrink: 0; }
    li span { flex: 1; font-size: 0.95rem; line-height: 1.4; }
    .swipe-delete {
      position: absolute; right: 0; top: 0; bottom: 0; width: 80px;
      background: #e55; color: #fff; border: none; cursor: pointer;
      font-size: 0.875rem; font-weight: 500;
      display: flex; align-items: center; justify-content: center;
      border-radius: 0 8px 8px 0;
    }
    p.empty { color: #999; font-size: 0.9rem; }
    .drag-handle { color: #ccc; cursor: grab; font-size: 1.1rem; padding: 0 0 0 0.15rem; flex: none; touch-action: none; user-select: none; line-height: 1; }
    .drag-handle:active { cursor: grabbing; }
    li.dragging { opacity: 0.3; }
  </style>
</head>
<body>
  <main>
    <h1>simple_todo</h1>
    <div class="tabs">
      <a href="/" class="tab-btn">Tasks</a>
      <a href="/?tab=reading" class="tab-btn">Reading List</a>
      <a href="/house" class="tab-btn active">House Projects</a>
    </div>
    <form id="add-form">
      <input type="text" id="text-input" placeholder="Add a house project…" autocomplete="off">
      <button type="submit" class="add">Add</button>
    </form>
    <ul id="list"></ul>
    <p class="empty" id="empty" hidden>No house projects yet.</p>
  </main>

  <script>
    const list  = document.getElementById('list');
    const empty = document.getElementById('empty');
    const form  = document.getElementById('add-form');
    const input = document.getElementById('text-input');

    async function loadProjects() {
      const items = await fetch('/house-projects').then(r => r.json());
      renderProjects(items);
    }

    function renderProjects(items) {
      list.innerHTML = '';
      empty.hidden = items.length > 0;
      items.forEach(t => {
        const li = document.createElement('li');
        li.dataset.id = t.id;
        if (t.done) li.classList.add('done');

        const swipeDel = document.createElement('button');
        swipeDel.className = 'swipe-delete';
        swipeDel.textContent = 'Delete';

        const row = document.createElement('div');
        row.className = 'item-row';

        const cb = document.createElement('input');
        cb.type = 'checkbox';
        cb.checked = t.done;
        cb.onchange = async () => {
          await fetch(`/house-projects/${t.id}`, {
            method: 'PATCH',
            headers: {'content-type': 'application/json'},
            body: JSON.stringify({ done: cb.checked }),
          });
          loadProjects();
        };

        const span = document.createElement('span');
        span.textContent = t.text;

        const handle = document.createElement('span');
        handle.className = 'drag-handle';
        handle.textContent = '⠿';

        row.append(cb, span, handle);
        li.append(swipeDel, row);
        list.appendChild(li);
      });
    }

    form.onsubmit = async e => {
      e.preventDefault();
      const text = input.value.trim();
      if (!text) return;
      await fetch('/house-projects', {
        method: 'POST',
        headers: {'content-type': 'application/json'},
        body: JSON.stringify({ text }),
      });
      input.value = '';
      loadProjects();
    };

    // ── Drag-to-reorder ────────────────────────────────────────────────────────
    function makeSortable(ul, endpoint) {
      let dragging = null, ghost = null, lastY = 0;

      function moveDrag(y) {
        if (!dragging) return;
        ghost.style.top = (parseFloat(ghost.style.top) + y - lastY) + 'px';
        lastY = y;
        const siblings = [...ul.querySelectorAll('li:not(.dragging)')];
        let target = null;
        for (const s of siblings) {
          const r = s.getBoundingClientRect();
          if (y < r.top + r.height / 2) { target = s; break; }
        }
        if (target) ul.insertBefore(dragging, target);
        else ul.appendChild(dragging);
      }

      function endDrag() {
        if (!dragging) return;
        dragging.classList.remove('dragging');
        ghost.remove(); ghost = null;
        const ids = [...ul.querySelectorAll('li')].map(li => parseInt(li.dataset.id));
        fetch(endpoint, { method: 'PUT', headers: {'content-type':'application/json'}, body: JSON.stringify({ids}) });
        document.removeEventListener('mousemove', onMM);
        document.removeEventListener('mouseup', endDrag);
        document.removeEventListener('touchmove', onTM);
        document.removeEventListener('touchend', endDrag);
        dragging = null;
      }

      function onMM(e) { moveDrag(e.clientY); }
      function onTM(e) { e.preventDefault(); moveDrag(e.touches[0].clientY); }

      function startDrag(li, y) {
        ul.querySelectorAll('li[data-open="1"]').forEach(item => {
          const r = item.querySelector('.item-row');
          if (r) { r.style.transition = ''; r.style.transform = ''; }
          item.dataset.open = '';
        });
        dragging = li; lastY = y;
        const r = li.getBoundingClientRect();
        ghost = li.cloneNode(true);
        const ghostDel = ghost.querySelector('.swipe-delete');
        if (ghostDel) ghostDel.style.display = 'none';
        ghost.style.cssText = `position:fixed;top:${r.top}px;left:${r.left}px;width:${r.width}px;opacity:0.7;pointer-events:none;z-index:9999;box-shadow:0 4px 12px rgba(0,0,0,0.15);border-radius:8px;`;
        document.body.appendChild(ghost);
        li.classList.add('dragging');
      }

      ul.addEventListener('mousedown', e => {
        if (!e.target.closest('.drag-handle')) return;
        e.preventDefault();
        startDrag(e.target.closest('li'), e.clientY);
        document.addEventListener('mousemove', onMM);
        document.addEventListener('mouseup', endDrag);
      });

      ul.addEventListener('touchstart', e => {
        if (!e.target.closest('.drag-handle')) return;
        e.preventDefault();
        startDrag(e.target.closest('li'), e.touches[0].clientY);
        document.addEventListener('touchmove', onTM, {passive: false});
        document.addEventListener('touchend', endDrag);
      }, {passive: false});
    }

    function makeSwipeable(ul, deleteUrl, reload) {
      let target = null, startX = 0, startY = 0, dirLocked = false;
      const W = 80;

      function getRow(li) { return li.querySelector('.item-row'); }
      function setX(li, x) {
        const row = getRow(li);
        if (row) row.style.transform = x ? `translateX(${-x}px)` : '';
      }
      function snap(li, open) {
        const row = getRow(li);
        if (!row) return;
        row.style.transition = 'transform 0.2s ease';
        setX(li, open ? W : 0);
        li.dataset.open = open ? '1' : '';
        setTimeout(() => { row.style.transition = ''; }, 200);
      }
      function closeAll(except) {
        ul.querySelectorAll('li[data-open="1"]').forEach(li => {
          if (li !== except) snap(li, false);
        });
      }

      ul.addEventListener('touchstart', e => {
        const li = e.target.closest('li');
        if (!li || e.target.closest('.drag-handle')) return;
        target = li; startX = e.touches[0].clientX; startY = e.touches[0].clientY; dirLocked = false;
        closeAll(li);
      }, {passive: true});

      ul.addEventListener('touchmove', e => {
        if (!target) return;
        const dx = e.touches[0].clientX - startX;
        const dy = e.touches[0].clientY - startY;
        if (!dirLocked) {
          if (Math.abs(dy) > Math.abs(dx) + 3) { target = null; return; }
          if (Math.abs(dx) > 5) dirLocked = true; else return;
        }
        const base = target.dataset.open === '1' ? W : 0;
        setX(target, Math.max(0, Math.min(W, base - dx)));
      }, {passive: true});

      ul.addEventListener('touchend', e => {
        if (!target) return;
        const dx = e.changedTouches[0].clientX - startX;
        snap(target, target.dataset.open === '1' ? dx > -(W / 2) : -dx > W / 2);
        target = null;
      });

      ul.addEventListener('click', async e => {
        const btn = e.target.closest('.swipe-delete');
        if (btn) {
          const li = btn.closest('li');
          await fetch(`${deleteUrl}/${li.dataset.id}`, { method: 'DELETE' });
          reload();
        } else {
          closeAll(null);
        }
      });

      document.addEventListener('touchstart', e => {
        if (!ul.contains(e.target)) closeAll(null);
      }, {passive: true});
    }

    loadProjects();
    makeSortable(list, '/house-projects/reorder');
    makeSwipeable(list, '/house-projects', loadProjects);
  </script>
</body>
</html>"#;
    html_response(html)
}

// ── House project handlers ─────────────────────────────────────────────────────

async fn list_house(State(state): State<AppState>) -> impl IntoResponse {
    let tasks = state.house.lock().unwrap().load().unwrap_or_default();
    Json(tasks)
}

async fn create_house(
    State(state): State<AppState>,
    Json(body): Json<CreateTaskBody>,
) -> impl IntoResponse {
    let store = state.house.lock().unwrap();
    let mut tasks = store.load().unwrap_or_default();
    let id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let task = Task::new(id, body.text);
    tasks.push(task.clone());
    store.save(&tasks).unwrap();
    (StatusCode::CREATED, Json(task))
}

async fn update_house(
    State(state): State<AppState>,
    Path(id): Path<u32>,
    Json(body): Json<UpdateTaskBody>,
) -> impl IntoResponse {
    let store = state.house.lock().unwrap();
    let mut tasks = store.load().unwrap_or_default();
    match tasks.iter_mut().find(|t| t.id == id) {
        Some(t) => {
            t.done = body.done;
            let updated = t.clone();
            store.save(&tasks).unwrap();
            (StatusCode::OK, Json(updated)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody { error: format!("no house project #{id}") }),
        )
            .into_response(),
    }
}

async fn delete_house(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let store = state.house.lock().unwrap();
    let mut tasks = store.load().unwrap_or_default();
    match tasks.iter().position(|t| t.id == id) {
        Some(pos) => {
            tasks.remove(pos);
            store.save(&tasks).unwrap();
            StatusCode::NO_CONTENT.into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody { error: format!("no house project #{id}") }),
        )
            .into_response(),
    }
}

// ── Reorder handlers ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ReorderBody {
    ids: Vec<u32>,
}

async fn reorder_tasks(
    State(state): State<AppState>,
    Json(body): Json<ReorderBody>,
) -> impl IntoResponse {
    let store = state.storage.lock().unwrap();
    let mut tasks = store.load().unwrap_or_default();
    tasks.sort_by_key(|t| body.ids.iter().position(|&id| id == t.id).unwrap_or(usize::MAX));
    store.save(&tasks).unwrap();
    StatusCode::OK
}

async fn reorder_house(
    State(state): State<AppState>,
    Json(body): Json<ReorderBody>,
) -> impl IntoResponse {
    let store = state.house.lock().unwrap();
    let mut tasks = store.load().unwrap_or_default();
    tasks.sort_by_key(|t| body.ids.iter().position(|&id| id == t.id).unwrap_or(usize::MAX));
    store.save(&tasks).unwrap();
    StatusCode::OK
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request};
    use http_body_util::BodyExt;
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn test_app(home: &TempDir) -> Router {
        std::env::set_var("HOME", home.path());
        let storage = Arc::new(Mutex::new(JsonStorage::default()));
        let reading = Arc::new(Mutex::new(JsonReadingList::default()));
        let house = Arc::new(Mutex::new(JsonStorage::new(home.path().join("house_projects.json"))));
        build_router(storage, reading, house)
    }

    fn test_app_with_reading(home: &TempDir) -> (Router, SharedReadingList) {
        std::env::set_var("HOME", home.path());
        let storage = Arc::new(Mutex::new(JsonStorage::default()));
        let reading = Arc::new(Mutex::new(JsonReadingList::default()));
        let house = Arc::new(Mutex::new(JsonStorage::new(home.path().join("house_projects.json"))));
        let app = build_router(Arc::clone(&storage), Arc::clone(&reading), house);
        (app, reading)
    }

    async fn body_string(body: Body) -> String {
        let bytes = body.collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    // ── GET / ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn index_returns_200() {
        let home = TempDir::new().unwrap();
        let app = test_app(&home);
        let resp = app
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn index_is_html() {
        let home = TempDir::new().unwrap();
        let app = test_app(&home);
        let resp = app
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
        let body = body_string(resp.into_body()).await;
        assert!(body.contains("simple_todo"));
        assert!(body.contains("/tasks"));
        assert!(body.contains("Reading List"));
    }

    // ── GET /tasks ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_empty_returns_empty_array() {
        let home = TempDir::new().unwrap();
        let app = test_app(&home);
        let resp = app
            .oneshot(Request::get("/tasks").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp.into_body()).await;
        assert_eq!(body.trim(), "[]");
    }

    // ── POST /tasks ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_task_returns_201_with_task() {
        let home = TempDir::new().unwrap();
        let app = test_app(&home);
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"Buy milk"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = body_string(resp.into_body()).await;
        assert!(body.contains("Buy milk"));
        assert!(body.contains(r#""done":false"#));
    }

    #[tokio::test]
    async fn create_assigns_sequential_ids() {
        let home = TempDir::new().unwrap();
        let storage = Arc::new(Mutex::new({
            std::env::set_var("HOME", home.path());
            JsonStorage::default()
        }));
        let reading = Arc::new(Mutex::new(JsonReadingList::default()));
        let house = Arc::new(Mutex::new(JsonStorage::new(home.path().join("house_projects.json"))));
        let app = build_router(Arc::clone(&storage), Arc::clone(&reading), house);

        let r1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"First"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let b1 = body_string(r1.into_body()).await;
        assert!(b1.contains(r#""id":1"#));

        let r2 = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"Second"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let b2 = body_string(r2.into_body()).await;
        assert!(b2.contains(r#""id":2"#));
    }

    // ── PATCH /tasks/{id} ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn patch_marks_task_done() {
        let home = TempDir::new().unwrap();
        let storage = Arc::new(Mutex::new({
            std::env::set_var("HOME", home.path());
            JsonStorage::default()
        }));
        let reading = Arc::new(Mutex::new(JsonReadingList::default()));
        let house = Arc::new(Mutex::new(JsonStorage::new(home.path().join("house_projects.json"))));
        let app = build_router(Arc::clone(&storage), Arc::clone(&reading), house);

        app.clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"Test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri("/tasks/1")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"done":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp.into_body()).await;
        assert!(body.contains(r#""done":true"#));
    }

    #[tokio::test]
    async fn patch_nonexistent_returns_404() {
        let home = TempDir::new().unwrap();
        let app = test_app(&home);
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri("/tasks/99")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"done":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ── DELETE /tasks/{id} ────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_task_returns_204() {
        let home = TempDir::new().unwrap();
        let storage = Arc::new(Mutex::new({
            std::env::set_var("HOME", home.path());
            JsonStorage::default()
        }));
        let reading = Arc::new(Mutex::new(JsonReadingList::default()));
        let house = Arc::new(Mutex::new(JsonStorage::new(home.path().join("house_projects.json"))));
        let app = build_router(Arc::clone(&storage), Arc::clone(&reading), house);

        app.clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"Delete me"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/tasks/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_nonexistent_returns_404() {
        let home = TempDir::new().unwrap();
        let app = test_app(&home);
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/tasks/42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ── GET /reading ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_reading_empty_returns_empty_array() {
        let home = TempDir::new().unwrap();
        let app = test_app(&home);
        let resp = app
            .oneshot(Request::get("/reading").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp.into_body()).await;
        assert_eq!(body.trim(), "[]");
    }

    // ── POST /reading ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_reading_fallback_on_bad_url() {
        let home = TempDir::new().unwrap();
        // localhost:1 is almost certainly not listening — fetch will fail,
        // we should fall back gracefully and still get 201.
        let app = test_app(&home);
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/reading")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"url":"http://localhost:1/page"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = body_string(resp.into_body()).await;
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["id"], 1);
        assert_eq!(v["read"], false);
        // title falls back to hostname
        assert!(!v["title"].as_str().unwrap().is_empty());
    }

    // ── PATCH /reading/{id} ───────────────────────────────────────────────────

    #[tokio::test]
    async fn patch_reading_marks_as_read() {
        let home = TempDir::new().unwrap();
        let (app, reading) = test_app_with_reading(&home);

        // Seed an item directly
        {
            let rl = reading.lock().unwrap();
            rl.save(&[ReadingItem {
                id: 1,
                url: "https://example.com".into(),
                title: "Example".into(),
                read: false,
            }])
            .unwrap();
        }

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri("/reading/1")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"read":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp.into_body()).await;
        assert!(body.contains(r#""read":true"#));
    }

    // ── DELETE /reading/{id} ──────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_reading_returns_204() {
        let home = TempDir::new().unwrap();
        let (app, reading) = test_app_with_reading(&home);

        {
            let rl = reading.lock().unwrap();
            rl.save(&[ReadingItem {
                id: 1,
                url: "https://example.com".into(),
                title: "Example".into(),
                read: false,
            }])
            .unwrap();
        }

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/reading/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_reading_nonexistent_returns_404() {
        let home = TempDir::new().unwrap();
        let app = test_app(&home);
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/reading/99")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ── GET /reading/{id}/view ────────────────────────────────────────────────

    #[tokio::test]
    async fn view_reading_nonexistent_returns_404() {
        let home = TempDir::new().unwrap();
        let app = test_app(&home);
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/reading/99/view")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn view_reading_returns_html() {
        let home = TempDir::new().unwrap();
        let (app, reading) = test_app_with_reading(&home);

        {
            let rl = reading.lock().unwrap();
            rl.save(&[ReadingItem {
                id: 1,
                url: "http://localhost:1/unreachable".into(),
                title: "Test Page".into(),
                read: false,
            }])
            .unwrap();
        }

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/reading/1/view")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
        let body = body_string(resp.into_body()).await;
        assert!(body.contains("Test Page"));
        assert!(body.contains("&larr; Back"));
    }

    // ── Unit: content extraction helpers ─────────────────────────────────────

    #[test]
    fn extract_title_finds_title_tag() {
        let html = "<html><head><title>Hello World</title></head><body></body></html>";
        assert_eq!(extract_title(html).unwrap(), "Hello World");
    }

    #[test]
    fn extract_title_returns_none_on_missing() {
        let html = "<html><body>no title here</body></html>";
        assert!(extract_title(html).is_none());
    }

    #[test]
    fn hostname_from_url_extracts_host() {
        assert_eq!(hostname_from_url("https://example.com/path"), "example.com");
        assert_eq!(hostname_from_url("http://localhost:8080/"), "localhost:8080");
    }

    #[test]
    fn strip_scripts_styles_removes_both() {
        let html = r#"<html><head><style>body{}</style><script>alert(1)</script></head><body>hello</body></html>"#;
        let clean = strip_scripts_styles(html);
        assert!(!clean.contains("<style"));
        assert!(!clean.contains("<script"));
        assert!(clean.contains("hello"));
    }
}
