//! Celestedle — a daily Celeste-entity guessing game (Wordle-style).
//!
//! One entity is deterministically picked per UTC day. The player guesses
//! entities; each guess is compared attribute by attribute against the
//! daily one and rendered as a colored table row (green exact, orange
//! partial, red wrong, arrows for the chapter). Entirely DB-free: the
//! dataset lives in this file, sourced from the official wiki's Entities
//! table (celeste.ink/wiki/Entities).

use axum::{
    extract::State,
    response::Html,
};
use serde::Serialize;
use std::sync::Arc;

use crate::AppState;

pub struct Entity {
    pub name: &'static str,
    /// enemy | hazard | block | gimmick | collectible
    pub category: &'static str,
    /// 0 = Prologue, 1–8 = chapters, 9 = Farewell.
    pub chapter: i64,
    /// Kills Madeline on contact.
    pub harmful: bool,
    /// Can be stood on.
    pub solid: bool,
    pub colors: &'static [&'static str],
}

/// First-appearance chapters follow the wiki's "Chapters found in" column.
const ENTITIES: &[Entity] = &[
    Entity { name: "Spring", category: "gimmick", chapter: 1, harmful: false, solid: false, colors: &["orange"] },
    Entity { name: "Strawberry", category: "collectible", chapter: 1, harmful: false, solid: false, colors: &["red"] },
    Entity { name: "Golden Strawberry", category: "collectible", chapter: 1, harmful: false, solid: false, colors: &["yellow"] },
    Entity { name: "Cassette", category: "collectible", chapter: 1, harmful: false, solid: false, colors: &["blue"] },
    Entity { name: "Cassette Block", category: "block", chapter: 1, harmful: false, solid: true, colors: &["blue", "pink"] },
    Entity { name: "Crystal Heart", category: "collectible", chapter: 1, harmful: false, solid: false, colors: &["blue", "red", "yellow"] },
    Entity { name: "Crumble Block", category: "block", chapter: 1, harmful: false, solid: true, colors: &["gray"] },
    Entity { name: "Falling Block", category: "block", chapter: 1, harmful: false, solid: true, colors: &["gray"] },
    Entity { name: "Zip Mover", category: "block", chapter: 1, harmful: false, solid: true, colors: &["gray", "red"] },
    Entity { name: "Refill", category: "gimmick", chapter: 1, harmful: false, solid: false, colors: &["green"] },
    Entity { name: "Spikes", category: "hazard", chapter: 1, harmful: true, solid: false, colors: &["gray"] },
    Entity { name: "Touch Switch", category: "gimmick", chapter: 1, harmful: false, solid: false, colors: &["blue"] },
    Entity { name: "Dream Block", category: "block", chapter: 2, harmful: false, solid: true, colors: &["black"] },
    Entity { name: "Badeline Chaser", category: "enemy", chapter: 2, harmful: true, solid: false, colors: &["purple"] },
    Entity { name: "Dust Bunny", category: "hazard", chapter: 3, harmful: true, solid: false, colors: &["black", "red"] },
    Entity { name: "Oshiro Boss", category: "enemy", chapter: 3, harmful: true, solid: false, colors: &["white", "blue"] },
    Entity { name: "Key", category: "collectible", chapter: 3, harmful: false, solid: false, colors: &["yellow"] },
    Entity { name: "Water", category: "gimmick", chapter: 3, harmful: false, solid: false, colors: &["blue"] },
    Entity { name: "Cloud", category: "block", chapter: 4, harmful: false, solid: true, colors: &["white", "pink"] },
    Entity { name: "Snowball", category: "hazard", chapter: 4, harmful: true, solid: false, colors: &["white"] },
    Entity { name: "Spinner", category: "hazard", chapter: 4, harmful: true, solid: false, colors: &["white", "blue"] },
    Entity { name: "Move Block", category: "block", chapter: 4, harmful: false, solid: true, colors: &["black", "blue"] },
    Entity { name: "Green Booster", category: "gimmick", chapter: 4, harmful: false, solid: false, colors: &["green"] },
    Entity { name: "Seeker", category: "enemy", chapter: 5, harmful: true, solid: false, colors: &["green", "gray"] },
    Entity { name: "Red Booster", category: "gimmick", chapter: 5, harmful: false, solid: false, colors: &["red"] },
    Entity { name: "Swap Block", category: "block", chapter: 5, harmful: false, solid: true, colors: &["red"] },
    Entity { name: "Theo Crystal", category: "gimmick", chapter: 5, harmful: false, solid: false, colors: &["blue"] },
    Entity { name: "Blade", category: "hazard", chapter: 5, harmful: true, solid: false, colors: &["gray"] },
    Entity { name: "Feather", category: "gimmick", chapter: 6, harmful: false, solid: false, colors: &["yellow"] },
    Entity { name: "Bumper", category: "gimmick", chapter: 6, harmful: false, solid: false, colors: &["brown", "blue"] },
    Entity { name: "Kevin", category: "block", chapter: 6, harmful: false, solid: true, colors: &["brown", "blue"] },
    Entity { name: "Badeline Boss", category: "enemy", chapter: 6, harmful: true, solid: false, colors: &["purple", "red"] },
    Entity { name: "Fireball", category: "hazard", chapter: 8, harmful: true, solid: false, colors: &["red", "orange"] },
    Entity { name: "Ice Ball", category: "hazard", chapter: 8, harmful: true, solid: false, colors: &["blue", "white"] },
    Entity { name: "Core Block", category: "block", chapter: 8, harmful: false, solid: true, colors: &["red", "blue"] },
    Entity { name: "Jellyfish", category: "gimmick", chapter: 9, harmful: false, solid: false, colors: &["white", "pink"] },
    Entity { name: "Pufferfish", category: "gimmick", chapter: 9, harmful: false, solid: false, colors: &["orange"] },
    Entity { name: "Lightning", category: "hazard", chapter: 9, harmful: true, solid: false, colors: &["purple"] },
    Entity { name: "Moon Block", category: "block", chapter: 9, harmful: false, solid: true, colors: &["white"] },
    Entity { name: "Generator", category: "gimmick", chapter: 9, harmful: false, solid: false, colors: &["gray", "yellow"] },
];

/// Today's entity, the same for everyone all (UTC) day: hash the day
/// number so consecutive days don't walk the list in order.
fn daily_index() -> usize {
    let days = chrono::Utc::now()
        .date_naive()
        .signed_duration_since(chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
        .num_days() as u64;
    (days.wrapping_mul(2654435761) % ENTITIES.len() as u64) as usize
}

fn chapter_label(ch: i64) -> String {
    match ch {
        0 => "Pr.".to_string(),
        9 => "9 (Farewell)".to_string(),
        n => n.to_string(),
    }
}

fn yes_no(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

#[derive(Serialize)]
struct PaletteEntity {
    id: usize,
    name: &'static str,
}

/// GET /celestedle — the game pane, loaded into #content. Ships the
/// entity names (nothing more — the answer stays server-side) for the
/// guess palette.
pub async fn page(State(state): State<Arc<AppState>>) -> Html<String> {
    let mut list: Vec<PaletteEntity> = ENTITIES
        .iter()
        .enumerate()
        .map(|(id, e)| PaletteEntity { id, name: e.name })
        .collect();
    list.sort_by_key(|e| e.name);

    let mut ctx = tera::Context::new();
    ctx.insert("entities", &list);
    Html(state.tera.render("partials/celestedle.html", &ctx).unwrap())
}

#[derive(Serialize)]
struct GuessCell {
    text: String,
    class: &'static str,
}

#[derive(serde::Deserialize)]
pub struct GuessForm {
    pub id: String,
}

/// POST /celestedle/guess — compare the guessed entity against today's
/// and return one colored table row, prepended to the results by htmx.
pub async fn guess(
    State(state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<GuessForm>,
) -> Html<String> {
    let Some(guess) = form.id.parse::<usize>().ok().and_then(|i| ENTITIES.get(i)) else {
        return Html(String::new());
    };
    let daily = &ENTITIES[daily_index()];
    let win = std::ptr::eq(guess, daily);

    let chapter_text = if guess.chapter == daily.chapter {
        chapter_label(guess.chapter)
    } else if daily.chapter > guess.chapter {
        format!("{} \u{2191}", chapter_label(guess.chapter))
    } else {
        format!("{} \u{2193}", chapter_label(guess.chapter))
    };

    let color_class = {
        let shared = guess.colors.iter().filter(|c| daily.colors.contains(c)).count();
        if shared == guess.colors.len() && guess.colors.len() == daily.colors.len() {
            "ok"
        } else if shared > 0 {
            "partial"
        } else {
            "no"
        }
    };

    let eq = |same: bool| if same { "ok" } else { "no" };
    let cells = vec![
        GuessCell { text: guess.name.to_string(), class: if win { "ok" } else { "neutral" } },
        GuessCell { text: guess.category.to_string(), class: eq(guess.category == daily.category) },
        GuessCell { text: chapter_text, class: eq(guess.chapter == daily.chapter) },
        GuessCell { text: yes_no(guess.harmful).to_string(), class: eq(guess.harmful == daily.harmful) },
        GuessCell { text: yes_no(guess.solid).to_string(), class: eq(guess.solid == daily.solid) },
        GuessCell { text: guess.colors.join(", "), class: color_class },
    ];

    let mut ctx = tera::Context::new();
    ctx.insert("cells", &cells);
    ctx.insert("win", &win);
    Html(
        state
            .tera
            .render("partials/celestedle_row.html", &ctx)
            .unwrap(),
    )
}
