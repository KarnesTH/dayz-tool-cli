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
