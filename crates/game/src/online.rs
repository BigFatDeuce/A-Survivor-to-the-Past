//! Classement en ligne : lecture du top 100 d'un stage (par version du jeu) et publication d'un score.
//! Le jeu ne contacte le serveur que quand le joueur ouvre l'onglet Monde ou publie un score.
//! Sur ordinateur : requetes HTTPS en arriere-plan ; dans le navigateur : `fetch` sur le meme domaine.
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Adresse du service (le site du jeu) ; `ASTP_SERVER` la remplace pour les essais (serveur local).
#[cfg(not(target_arch = "wasm32"))]
fn base() -> String {
    std::env::var("ASTP_SERVER").unwrap_or_else(|_| "https://a-survivor-to-the-past.holy-c.men".to_string())
}

/// Dans le navigateur, le jeu est servi par le meme domaine que le classement.
#[cfg(target_arch = "wasm32")]
fn base() -> String {
    String::new()
}

#[derive(Clone, Debug, Deserialize)]
pub struct Entry {
    pub rank: u32,
    pub name: String,
    pub score: u64,
    #[serde(default)]
    pub character: String,
    #[serde(default)]
    pub time: f32,
    #[serde(default)]
    pub kills: u32,
    #[serde(default)]
    pub won: bool,
    #[serde(default)]
    pub difficulty: usize,
    #[serde(default)]
    pub title: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Board {
    pub version: String,
    /// Versions qui ont des scores (de la plus recente a la plus ancienne).
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default)]
    pub scores: Vec<Entry>,
}

/// Score envoye : le serveur recalcule le score et refuse les valeurs impossibles.
#[derive(Clone, Debug, Serialize)]
pub struct Submission {
    pub map: String,
    pub version: String,
    pub name: String,
    pub score: u64,
    pub time: f32,
    pub kills: u32,
    pub level: u32,
    pub bosses: u32,
    pub won: bool,
    pub character: String,
    pub difficulty: usize,
    pub mult: f32,
    /// 0.6.3 : titre offert a la Grande Fee (0 : aucun).
    pub title: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SubmitReply {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub rank: Option<u32>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Status<T> {
    Idle,
    Busy,
    Done(T),
    Failed(String),
}

/// Dernier classement demande : (stage, version demandee) et son etat.
type BoardSlot = Mutex<((String, String), Status<Board>)>;

#[derive(Clone)]
pub struct Online {
    board: Arc<BoardSlot>,
    submit: Arc<Mutex<Status<SubmitReply>>>,
}

impl Default for Online {
    fn default() -> Self {
        Online { board: Arc::new(Mutex::new(((String::new(), String::new()), Status::Idle))), submit: Arc::new(Mutex::new(Status::Idle)) }
    }
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

impl Online {
    /// Classement demande et son etat.
    pub fn board(&self) -> ((String, String), Status<Board>) {
        self.board.lock().map(|g| g.clone()).unwrap_or(((String::new(), String::new()), Status::Idle))
    }

    /// Demande le classement d'un stage (version vide : la plus recente du serveur).
    pub fn fetch(&self, map: &str, version: &str) {
        let key = (map.to_string(), version.to_string());
        if let Ok(mut g) = self.board.lock() {
            if g.0 == key && matches!(g.1, Status::Busy | Status::Done(_)) {
                return;
            }
            *g = (key.clone(), Status::Busy);
        }
        let url = format!("{}/api/scores?map={}&version={}", base(), enc(map), enc(version));
        let slot = self.board.clone();
        get_json::<Board>(url, move |r| {
            if let Ok(mut g) = slot.lock() {
                if g.0 == key {
                    g.1 = match r {
                        Ok(b) => Status::Done(b),
                        Err(e) => Status::Failed(e),
                    };
                }
            }
        });
    }

    /// Essais de l'interface : classement fictif.
    pub fn force(&self, map: &str, version: &str, status: Status<Board>) {
        if let Ok(mut g) = self.board.lock() {
            *g = ((map.to_string(), version.to_string()), status);
        }
    }

    /// Oublie le classement en memoire (nouvelle lecture au prochain affichage).
    pub fn refresh(&self) {
        if let Ok(mut g) = self.board.lock() {
            g.1 = Status::Idle;
        }
    }

    pub fn submit_status(&self) -> Status<SubmitReply> {
        self.submit.lock().map(|g| g.clone()).unwrap_or(Status::Idle)
    }

    pub fn clear_submit(&self) {
        if let Ok(mut g) = self.submit.lock() {
            *g = Status::Idle;
        }
    }

    pub fn submit(&self, s: Submission) {
        if let Ok(mut g) = self.submit.lock() {
            *g = Status::Busy;
        }
        let body = serde_json::to_string(&s).unwrap_or_default();
        let slot = self.submit.clone();
        post_json::<SubmitReply>(format!("{}/api/scores", base()), body, move |r| {
            if let Ok(mut g) = slot.lock() {
                *g = match r {
                    Ok(rep) if rep.ok => Status::Done(rep),
                    Ok(rep) => Status::Failed(rep.error.unwrap_or_else(|| "refused".into())),
                    Err(e) => Status::Failed(e),
                };
            }
        });
    }
}

// ---------------------------------------------------------------------------------------------
// Transport

#[cfg(not(target_arch = "wasm32"))]
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(12)))
        .http_status_as_error(false)
        .user_agent(concat!("A-Survivor-to-the-Past/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

#[cfg(not(target_arch = "wasm32"))]
fn get_json<T: for<'de> Deserialize<'de> + Send + 'static>(url: String, done: impl FnOnce(Result<T, String>) + Send + 'static) {
    std::thread::spawn(move || {
        let r = agent().get(&url).call().map_err(|e| e.to_string()).and_then(|mut res| {
            let status = res.status().as_u16();
            let text = res.body_mut().read_to_string().map_err(|e| e.to_string())?;
            if status >= 400 {
                return Err(format!("HTTP {status}"));
            }
            serde_json::from_str::<T>(&text).map_err(|e| e.to_string())
        });
        done(r);
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn post_json<T: for<'de> Deserialize<'de> + Send + 'static>(url: String, body: String, done: impl FnOnce(Result<T, String>) + Send + 'static) {
    std::thread::spawn(move || {
        let r = agent()
            .post(&url)
            .header("Content-Type", "application/json")
            .send(body.as_str())
            .map_err(|e| e.to_string())
            .and_then(|mut res| {
                let text = res.body_mut().read_to_string().map_err(|e| e.to_string())?;
                serde_json::from_str::<T>(&text).map_err(|e| e.to_string())
            });
        done(r);
    });
}

#[cfg(target_arch = "wasm32")]
async fn fetch_text(url: String, body: Option<String>) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    let window = web_sys::window().ok_or("window")?;
    let init = web_sys::RequestInit::new();
    if let Some(b) = &body {
        init.set_method("POST");
        init.set_body(&wasm_bindgen::JsValue::from_str(b));
        let h = web_sys::Headers::new().map_err(|_| "headers")?;
        h.set("Content-Type", "application/json").map_err(|_| "headers")?;
        init.set_headers(&h);
    }
    let req = web_sys::Request::new_with_str_and_init(&url, &init).map_err(|_| "request")?;
    let resp = JsFuture::from(window.fetch_with_request(&req)).await.map_err(|_| "offline".to_string())?;
    let resp: web_sys::Response = resp.dyn_into().map_err(|_| "response")?;
    let status = resp.status();
    let text = JsFuture::from(resp.text().map_err(|_| "text")?).await.map_err(|_| "text")?;
    let text = text.as_string().unwrap_or_default();
    if status >= 400 && body.is_none() {
        return Err(format!("HTTP {status}"));
    }
    Ok(text)
}

#[cfg(target_arch = "wasm32")]
fn get_json<T: for<'de> Deserialize<'de> + 'static>(url: String, done: impl FnOnce(Result<T, String>) + 'static) {
    wasm_bindgen_futures::spawn_local(async move {
        let r = fetch_text(url, None).await.and_then(|t| serde_json::from_str::<T>(&t).map_err(|e| e.to_string()));
        done(r);
    });
}

#[cfg(target_arch = "wasm32")]
fn post_json<T: for<'de> Deserialize<'de> + 'static>(url: String, body: String, done: impl FnOnce(Result<T, String>) + 'static) {
    wasm_bindgen_futures::spawn_local(async move {
        let r = fetch_text(url, Some(body)).await.and_then(|t| serde_json::from_str::<T>(&t).map_err(|e| e.to_string()));
        done(r);
    });
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Essai contre un serveur local : ASTP_SERVER=http://127.0.0.1:8787 cargo test -p game online -- --ignored
    #[test]
    #[ignore]
    fn submit_then_read() {
        let o = Online::default();
        let s = Submission {
            map: "lost_woods".into(),
            version: std::env::var("ASTP_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").into()),
            name: "Essai".into(),
            score: crate::profile::score(1200, 420.25, 1, false, 1.0),
            time: 420.25,
            kills: 1200,
            level: 30,
            bosses: 1,
            won: false,
            character: "link".into(),
            difficulty: 1,
            mult: 1.0,
            title: 0,
        };
        o.submit(s);
        let wait = |f: &dyn Fn() -> bool| {
            for _ in 0..100 {
                if f() {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            panic!("pas de reponse");
        };
        wait(&|| !matches!(o.submit_status(), Status::Busy | Status::Idle));
        let st = o.submit_status();
        assert!(matches!(&st, Status::Done(r) if r.rank.is_some()), "{st:?}");
        o.fetch("lost_woods", "");
        wait(&|| !matches!(o.board().1, Status::Busy | Status::Idle));
        let (_, b) = o.board();
        let Status::Done(b) = b else { panic!("{b:?}") };
        assert!(b.scores.iter().any(|e| e.name == "Essai"), "{b:?}");
    }
}
