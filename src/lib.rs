use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

use lazy_static::lazy_static;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub mod commands;
pub mod utils;

#[derive(Debug, Error, PartialEq)]
pub enum GuidError {
    #[error("Steam64ID must be 17 characters long")]
    InvalidLength,
    #[error("Steam64ID must start with 7656119")]
    InvalidPrefix,
    #[error("Steam64ID must contain only numeric characters")]
    InvalidCharacters,
}

pub type Result<T> = std::result::Result<T, GuidError>;

#[derive(Debug, Error, PartialEq)]
pub enum ConfigError {
    #[error("Failed to create the configuration file")]
    CreateFileError,
    #[error("Failed to read the configuration file")]
    ReadFileError,
    #[error("Failed to write the configuration file")]
    WriteFileError,
    #[error("Failed to parse the configuration file")]
    ParseError,
    #[error("Failed to find the configuration file")]
    OpenFileError,
    #[error("No active profile found")]
    NoActiveProfile,
    #[error("Failed to find the profile")]
    ProfileNotFoundError,
    #[error("Failed to serialize the value")]
    SerializeError,
}

#[derive(Debug, Error, PartialEq)]
pub enum DncError {
    #[error("Invalid time format. Use 'h' for hours or 'min' for minutes")]
    InvalidTimeFormat,
    #[error("Invalid time value. The time value must be a number.")]
    InvalidNumber,
    #[error("serverTimeAcceleration must be between 0.1 and 64.0")]
    InvalidTimeAcceleration,
    #[error("serverNightTimeAcceleration must be between 0.1 and 64.0")]
    InvalidNightTimeAcceleration,
}

#[derive(Debug, Error, PartialEq)]
pub enum ModError {
    #[error("Failed to find the mod")]
    NotFound,
    #[error("Failed to install the mod")]
    InstallError,
    #[error("Failed to uninstall the mod")]
    UninstallError,
    #[error("Failed to update the mod")]
    UpdateError,
    #[error("Failed to select mods")]
    SelectError,
    #[error("Failed to create destination folder")]
    CreateDirError,
    #[error("Failed to copy file")]
    CopyFileError,
    #[error("Failed to parse startup parameter")]
    ParseError,
    #[error("Failed to find the path")]
    PathError,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub profiles: Vec<Profile>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub workdir_path: String,
    pub workshop_path: String,
    pub installed_mods: Vec<Value>,
    pub is_active: bool,
}

lazy_static! {
    pub static ref THREAD_POOL: ThreadPool = ThreadPool::new(num_cpus::get());
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Box<dyn FnOnce() + Send>>,
}

type Job = Box<dyn FnOnce() + Send>;
type Receiver = Arc<Mutex<mpsc::Receiver<Job>>>;

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for _ in 0..size {
            workers.push(Worker::new(receiver.clone()));
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let task = Box::new(task);
        self.sender.send(task).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.sender.send(Box::new(|| {})).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Receiver) -> Worker {
        let thread = thread::spawn(move || loop {
            let task = receiver.lock().unwrap().recv();

            match task {
                Ok(task) => {
                    task();
                }
                Err(_) => {
                    break;
                }
            }
        });

        Worker {
            thread: Some(thread),
        }
    }
}

pub struct Mod {
    name: String,
}

impl Mod {
    pub fn short_name(&self) -> String {
        let mut short_name = String::new();
        let parts = self.name.split(|c| c == ' ' || c == '-' || c == '_');
        for part in parts {
            short_name.push_str(&part.chars().take(3).collect::<String>());
        }
        short_name
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Type {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "nominal")]
    pub nominal: i32,
    #[serde(rename = "lifetime")]
    pub lifetime: i32,
    #[serde(rename = "restock")]
    pub restock: i32,
    #[serde(rename = "min")]
    pub min: i32,
    #[serde(rename = "quantmin")]
    pub quantmin: i32,
    #[serde(rename = "quantmax")]
    pub quantmax: i32,
    #[serde(rename = "cost")]
    pub cost: i32,
    #[serde(rename = "flags")]
    pub flags: Flags,
    #[serde(rename = "category")]
    pub category: Category,
    #[serde(rename = "usage")]
    pub usage: Option<Vec<String>>,
    #[serde(rename = "tag")]
    pub tag: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Flags {
    #[serde(rename = "count_in_cargo")]
    pub count_in_cargo: i32,
    #[serde(rename = "count_in_hoarder")]
    pub count_in_hoarder: i32,
    #[serde(rename = "count_in_map")]
    pub count_in_map: i32,
    #[serde(rename = "count_in_player")]
    pub count_in_player: i32,
    #[serde(rename = "crafted")]
    pub crafted: i32,
    #[serde(rename = "deloot")]
    pub deloot: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Category {
    #[serde(rename = "name")]
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct SpawnableType {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "attachments")]
    pub attachments: Option<Vec<Attachment>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Attachment {
    #[serde(rename = "chance")]
    pub chance: f64,
    #[serde(rename = "item")]
    pub item: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Event {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "waves")]
    pub waves: i32,
    #[serde(rename = "nominal")]
    pub nominal: i32,
    #[serde(rename = "min")]
    pub min: i32,
    #[serde(rename = "max")]
    pub max: i32,
    #[serde(rename = "lifetime")]
    pub lifetime: i32,
    #[serde(rename = "restock")]
    pub restock: i32,
    #[serde(rename = "saferadius")]
    pub saferadius: i32,
    #[serde(rename = "distanceradius")]
    pub distanceraduis: i32,
    #[serde(rename = "cleanupradius")]
    pub cleanupradius: i32,
    #[serde(rename = "flags")]
    pub flags: Flags,
    #[serde(rename = "position")]
    pub position: String,
    #[serde(rename = "limit")]
    pub limit: String,
    #[serde(rename = "active")]
    pub active: i32,
    #[serde(rename = "children")]
    pub children: Option<Vec<Child>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Child {
    #[serde(rename = "lootmax")]
    pub lootmax: i32,
    #[serde(rename = "lootmin")]
    pub lootmin: i32,
    #[serde(rename = "max")]
    pub max: i32,
    #[serde(rename = "min")]
    pub min: i32,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Serialize)]
#[serde(rename = "types")]
pub struct TypesWrapper {
    #[serde(rename = "type")]
    types: Vec<Type>,
}

#[derive(Debug, Serialize)]
#[serde(rename = "spawnabletypes")]
pub struct SpawnableTypesWrapper {
    #[serde(rename = "type")]
    spawnable_types: Vec<SpawnableType>,
}

#[derive(Debug, Serialize)]
#[serde(rename = "events")]
pub struct EventsWrapper {
    #[serde(rename = "event")]
    events: Vec<Event>,
}
