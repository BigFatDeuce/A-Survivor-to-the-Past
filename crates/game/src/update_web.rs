//! Version web : le site sert toujours la derniere version, il n'y a rien a mettre a jour.
//! Memes types et fonctions que `update.rs` (ecran titre, options, reglages).
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};


/// Reglage du joueur (garde pour que les reglages restent compatibles).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum UpdateMode {
    #[default]
    Auto,
    Ask,
    Off,
}

impl UpdateMode {
    pub const ALL: [UpdateMode; 3] = [UpdateMode::Auto, UpdateMode::Ask, UpdateMode::Off];
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
}

impl Release {
    pub fn sample(version: &str) -> Release {
        Release { version: version.into() }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum Status {
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Downloading { version: String, done: u64, total: u64 },
    Ready(String),
    Failed(String),
}

pub struct Updater {
    status: std::cell::RefCell<Status>,
    #[allow(dead_code)]
    pub enabled: bool,
}

impl Updater {
    pub fn new(_root: &Path, _source: Option<PathBuf>) -> Updater {
        Updater { status: std::cell::RefCell::new(Status::Idle), enabled: false }
    }

    pub fn status(&self) -> Status {
        self.status.borrow().clone()
    }

    pub fn force_status(&self, s: Status) {
        *self.status.borrow_mut() = s;
    }

    pub fn check(&self) {}

    pub fn download(&self, _rel: Release) {}
}

pub fn launch_pending(_root: &Path) -> bool {
    false
}
