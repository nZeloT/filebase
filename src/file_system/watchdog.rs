/*
 * Copyright 2021 nzelot<leontsteiner@gmail.com>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

use new_mime_guess::MimeGuess;
use notify::{DebouncedEvent, RecursiveMode, Watcher};

use crate::file_system::storage::MediaItemMetadataStorage;
use crate::file_system::FileSystemError;
use std::thread;
use chrono::TimeZone;
use crate::file_system::thumbnail::Thumbnails;

#[derive(Debug, Clone)]
pub enum FilesystemWatchdogError {
    WatchdogError(String),
    ChannelError(String),
    StorageError(FileSystemError),
    ThumbnailError(FileSystemError),
    NoUtf8Filename(OsString),
    IoError(String),
    ExifError(String),
    ChronoError(String)
}

type Result<T> = std::result::Result<T, FilesystemWatchdogError>;

pub struct FileSystemWatchdogBuilder(
    FileSystemWatchdogData
);

struct FileSystemWatchdog(
    FileSystemWatchdogData,
    tokio::runtime::Runtime,
);

struct FileSystemWatchdogData {
    monitoring_dir: PathBuf,
    storage: MediaItemMetadataStorage,
    thumbnails : Thumbnails
}

impl FileSystemWatchdogBuilder {
    pub fn new(monitoring: &Path, storage: MediaItemMetadataStorage, thumbnails : Thumbnails) -> Self {
        FileSystemWatchdogBuilder(FileSystemWatchdogData {
            monitoring_dir: monitoring.to_path_buf(),
            storage,
            thumbnails
        })
    }

    pub fn launch(self) -> std::thread::JoinHandle<Result<()>> {
        let data = self.0;

        thread::spawn(move || {
            println!("Launching Watchdog");
            let rt = tokio::runtime::Runtime::new().expect("Failed to spawn new runtime in watchdog thread!");
            let wd = FileSystemWatchdog::new(data, rt);
            wd.watch()
        })
    }
}

impl FileSystemWatchdog {
    fn new(data: FileSystemWatchdogData, rt: tokio::runtime::Runtime) -> Self {
        FileSystemWatchdog(data, rt)
    }

    fn watch(self) -> Result<()> {
        println!("Scanning monitoring dir for existing files");
        self.scan_directory();

        let (tx, rx) = channel();

        let mut watchdog = match notify::watcher(tx, Duration::from_secs(5)) {
            Ok(w) => w,
            Err(e) => return Err(FilesystemWatchdogError::WatchdogError(e.to_string()))
        };

        if let Err(e) = watchdog.watch(self.0.monitoring_dir.to_path_buf(), RecursiveMode::NonRecursive) {
            return Err(FilesystemWatchdogError::WatchdogError(e.to_string()));
        }

        println!("Starting to watch for events on {:?}", self.0.monitoring_dir);

        loop {
            match rx.recv() {
                Ok(event) => {
                    match self.handle_event(event) {
                        Ok(_) => {}
                        Err(e) => println!("{:?}", e)
                    }
                }
                Err(err) => return Err(FilesystemWatchdogError::ChannelError(err.to_string()))
            }
        }
    }

    fn handle_event(&self, event: DebouncedEvent) -> Result<()> {
        match event {
            DebouncedEvent::NoticeWrite(pb) | DebouncedEvent::Write(pb) | DebouncedEvent::Create(pb) => {
                if !self.block_on(self.0.storage.is_path_known(&pb)) && pb.is_file() {
                    self.store_new_file(pb)
                } else {
                    Ok(())
                }
            }
            DebouncedEvent::NoticeRemove(pb) | DebouncedEvent::Remove(pb) => {
                self.block_on(self.0.storage.remove_if_known(&pb));
                Ok(())
            }
            DebouncedEvent::Chmod(_) => {
                println!("Watchdog: chmod");
                Ok(())
            }
            DebouncedEvent::Rename(_src, _dst) => panic!("Filesystem rename is not supported!"),
            DebouncedEvent::Rescan => panic!("Filesystem Rescan is not supported!"),
            DebouncedEvent::Error(err, _opt_pb) => {
                Err(FilesystemWatchdogError::WatchdogError(err.to_string()))
            }
        }
    }

    fn scan_directory(&self) {
        let dts = &self.0.monitoring_dir;
        for fs_entry in dts.read_dir().expect("Read dir failed!") {
            if let Ok(entry) = fs_entry {
                if entry.path().is_file() && !self.block_on(self.0.storage.is_path_known(entry.path().as_path())) {
                    self.store_new_file(entry.path()).expect("Error while storing newly found file!");
                }
            }
        }
    }

    fn store_new_file(&self, path: PathBuf) -> Result<()> {
        let fnm = path.file_name().expect("Given Path is no file!");
        if let Some(filename) = fnm.to_str() {
            let mime = MimeGuess::from_path(&path).first_or_octet_stream();
            let mime_type = mime.to_string();


            let creation_date = if mime.type_() == new_mime_guess::mime::IMAGE {
                match self.read_date_taken_from_exif(path.as_path()) {
                    Ok(date) => date,
                    Err(e) => {
                        println!("Reading EXIF data failed for reason '{:?}'; Falling back to file metadata.", e);
                        let created = path.metadata()?.created()?;
                        chrono::DateTime::<chrono::Utc>::from(created)
                    }
                }
            }else{
                let created = path.metadata()?.created()?;
                chrono::DateTime::<chrono::Utc>::from(created)
            };

            println!("Adding file {:?}", path);

            let r = self.block_on(
                self.0.storage.add_file(
                    path.as_path(),
                    String::from(filename),
                    mime_type,
                    creation_date)
            );

            match r {
                Ok(item) => {
                    let thumbnail_res = self.block_on(
                        self.0.thumbnails.load(
                            &item
                        )
                    );

                    match thumbnail_res {
                        Ok(_) => Ok(()),
                        Err(e) => Err(FilesystemWatchdogError::ThumbnailError(e))
                    }
                },
                Err(e) => Err(FilesystemWatchdogError::StorageError(e))

            }

        } else {
            Err(FilesystemWatchdogError::NoUtf8Filename(fnm.to_os_string()))
        }
    }

    fn read_date_taken_from_exif(&self, path : &Path) -> Result<chrono::DateTime<chrono::Utc>> {
        println!("Try reading file '{:?}'", path);
        let file = std::fs::File::open(path)?;
        let mut reader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();
        let exif = exifreader.read_from_container(&mut reader)?;

        println!("Reading EXIF data ...");

        //for f in exif.fields() {
        //    println!("\t{} {} {}", f.tag, f.ifd_num, f.display_value())
        //}

        match exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
            .or(exif.get_field(exif::Tag::DateTimeDigitized, exif::In::PRIMARY))
            .or(exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY))
            .and_then(|dt| {
                let date_str = dt.display_value().to_string();
                println!("Found DateTime in exif data; '{}'", date_str);
                match chrono::Utc.datetime_from_str(date_str.as_str(), "%Y-%m-%d %H:%M:%S") {
                    Ok(dt) => Some(dt),
                    Err(e) => {
                        println!("Encountered parse error => {}", e);
                        None
                    }
                }
            }) {
            Some(taken_on) => Ok(taken_on),
            None => Err(FilesystemWatchdogError::WatchdogError(format!("No date filed given in EXIF data!")))
        }
    }

    fn block_on<F: core::future::Future>(&self, future: F) -> F::Output {
        self.1.block_on(future)
    }
}

impl From<std::io::Error> for FilesystemWatchdogError {
    fn from(e: std::io::Error) -> Self {
        FilesystemWatchdogError::IoError(e.to_string())
    }
}

impl From<exif::Error> for FilesystemWatchdogError {
    fn from(e : exif::Error) -> Self {
        FilesystemWatchdogError::ExifError(e.to_string())
    }
}

impl From<chrono::ParseError> for FilesystemWatchdogError {
    fn from(e : chrono::ParseError) -> Self {
        FilesystemWatchdogError::ChronoError(e.to_string())
    }
}

