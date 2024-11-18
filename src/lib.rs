use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
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
    #[error("Failed to remove file")]
    RemoveFileError,
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
    job_count: Arc<AtomicUsize>,
}

type Job = Box<dyn FnOnce() + Send>;
type Receiver = Arc<Mutex<mpsc::Receiver<Job>>>;

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let job_count = Arc::new(AtomicUsize::new(0));

        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            workers.push(Worker::new(receiver.clone()));
        }

        ThreadPool {
            workers,
            sender,
            job_count,
        }
    }

    pub fn execute<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.job_count.fetch_add(1, Ordering::SeqCst);
        let job_count = self.job_count.clone();
        let task = Box::new(move || {
            task();
            job_count.fetch_sub(1, Ordering::SeqCst);
        });
        self.sender.send(task).unwrap();
    }

    pub fn wait(&self) {
        while self.job_count.load(Ordering::SeqCst) > 0 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
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
        let parts = self.name.split([' ', '-', '_']);
        for part in parts {
            short_name.push_str(&part.chars().take(3).collect::<String>().replace('@', ""));
        }
        short_name
    }
}

#[derive(Debug, Serialize)]
pub struct Types {
    #[serde(rename = "type")]
    pub items: Vec<Type>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Type {
    #[serde(rename = "@name", alias = "name")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nominal: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifetime: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restock: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantmin: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantmax: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Flags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Vec<Usage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<Vec<Tag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Vec<TypeValue>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Flags {
    #[serde(rename = "@count_in_cargo", alias = "count_in_cargo")]
    pub count_in_cargo: i32,
    #[serde(rename = "@count_in_hoarder", alias = "count_in_hoarder")]
    pub count_in_hoarder: i32,
    #[serde(rename = "@count_in_map", alias = "count_in_map")]
    pub count_in_map: i32,
    #[serde(rename = "@count_in_player", alias = "count_in_player")]
    pub count_in_player: i32,
    #[serde(rename = "@crafted", alias = "crafted")]
    pub crafted: i32,
    #[serde(rename = "@deloot", alias = "deloot")]
    pub deloot: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Category {
    #[serde(rename = "@name", alias = "name")]
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Usage {
    #[serde(rename = "@name", alias = "name")]
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Tag {
    #[serde(rename = "@name", alias = "name")]
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct TypeValue {
    #[serde(rename = "@name", alias = "name")]
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct SpawnableTypes {
    #[serde(rename = "type")]
    pub items: Vec<SpawnableType>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct SpawnableType {
    #[serde(rename = "@name", alias = "name")]
    pub name: String,
    pub attachments: Vec<Attachments>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Attachments {
    #[serde(rename = "@chance", alias = "chance")]
    pub chance: f64,
    pub item: Vec<Item>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Item {
    #[serde(rename = "@name", alias = "name")]
    pub name: String,
    #[serde(rename = "@chance", alias = "chance")]
    pub chance: f64,
}

#[derive(Debug, Serialize)]
pub struct Events {
    #[serde(rename = "event")]
    pub items: Vec<Event>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Event {
    #[serde(rename = "@name", alias = "name")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nominal: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifetime: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restock: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saferadius: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distanceraduis: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanupradius: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<EventFlags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Children>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename = "children")]
pub struct Children {
    #[serde(rename = "child")]
    pub items: Vec<Child>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename = "child")]
pub struct Child {
    #[serde(rename = "@lootmax", alias = "lootmax")]
    pub lootmax: i32,
    #[serde(rename = "@lootmin", alias = "lootmin")]
    pub lootmin: i32,
    #[serde(rename = "@max", alias = "max")]
    pub max: i32,
    #[serde(rename = "@min", alias = "min")]
    pub min: i32,
    #[serde(rename = "@type", alias = "type")]
    pub type_: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct EventFlags {
    #[serde(rename = "@deletable", alias = "deletable")]
    pub deletable: i32,
    #[serde(rename = "@init_random", alias = "init_random")]
    pub init_random: i32,
    #[serde(rename = "@remove_damaged", alias = "remove_damaged")]
    pub remove_damaged: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename = "types")]
pub struct TypesWrapper {
    #[serde(rename = "type")]
    pub types: Vec<Type>,
}

#[derive(Debug, Serialize)]
#[serde(rename = "spawnabletypes")]
pub struct SpawnableTypesWrapper {
    #[serde(rename = "type")]
    pub spawnable_types: Vec<SpawnableType>,
}

#[derive(Debug, Serialize)]
#[serde(rename = "events")]
pub struct EventsWrapper {
    #[serde(rename = "event")]
    pub events: Vec<Event>,
}

#[derive(Debug, Clone)]
pub struct ModChecksum {
    pub path: PathBuf,
    pub size: u64,
    pub hash: String,
}
