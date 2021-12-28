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

use std::path::{PathBuf, Path};
use std::sync::Arc;
use tokio::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::file_system::destinations::{FileSystemDestinations, FileSystemDestination};
use crate::file_system::model::{MediaItemMetadata};
use crate::file_system::storage::MediaItemMetadataStorage;
use crate::file_system::thumbnail::Thumbnails;

pub mod model;
pub mod watchdog;
mod storage;
mod destinations;
mod thumbnail;

type Result<T> = std::result::Result<T, FileSystemError>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FileSystemError {
    UnknownId(u64),
    UnknownPath(PathBuf),
    InvalidParameters(String),
    FileNotFound(PathBuf),
    IOError(String),
    NoParentDirectory(PathBuf),
    ImageError(String),
    Other(String),
    MultipleErrors(Vec<FileSystemError>),
}

#[derive(Clone)]
pub struct FileSystem(
    Arc<RwLock<FileSystemInternal>>
);

impl FileSystem {
    pub fn new<P: AsRef<Path>>(source_files: &Path, destination_config: P) -> Self {
        FileSystem(Arc::new(RwLock::new(FileSystemInternal {
            destinations: FileSystemDestinations::from_file(destination_config),
            storage: MediaItemMetadataStorage::new(),
            thumbnails: Thumbnails::new(source_files),
        })))
    }

    pub async fn launch_watchdog(&self, monitoring_dir: &Path) -> std::thread::JoinHandle<std::result::Result<(), watchdog::FilesystemWatchdogError>> {
        watchdog::FileSystemWatchdogBuilder::new(monitoring_dir,
                                                 self.0.read().await.storage.clone(),
                                                 self.0.read().await.thumbnails.clone(),
        )
            .launch()
    }

    pub async fn list_confirm_destinations(&self) -> Vec<FileSystemDestination> {
        println!("Listing known confirm destinations");
        self.0.read().await.destinations.list()
    }

    pub async fn list(&self) -> Result<Vec<MediaItemMetadata>> {
        self.0.read().await.list().await
    }

    pub async fn read(&self, id: u64) -> Result<Vec<u8>> {
        self.0.read().await.read(id).await
    }

    pub async fn discard(&self, ids: Vec<u64>) -> Result<()> {
        self.0.write().await.discard(ids).await
    }

    pub async fn discard_all(&self) -> Result<()> {
        self.0.write().await.discard_all().await
    }

    pub async fn confirm(&self, destination_id: &u64, ids: Vec<u64>) -> Result<()> {
        self.0.write().await.confirm(destination_id, ids).await
    }
}

struct FileSystemInternal {
    destinations: FileSystemDestinations,
    storage: MediaItemMetadataStorage,
    thumbnails: Thumbnails,
}

impl FileSystemInternal {
    pub async fn list(&self) -> Result<Vec<MediaItemMetadata>> {
        println!("Listing known items");
        self.storage.list_files().await
    }

    pub async fn read(&self, id: u64) -> Result<Vec<u8>> {
        println!("Reading requested thumbnail {}", id);
        self.thumbnails.get(&id).await
    }

    pub async fn discard(&self, ids: Vec<u64>) -> Result<()> {
        println!("Trying to discard items {:?}", ids);
        let mut failures = Vec::<FileSystemError>::new();
        for id in ids {
            match self.storage.get_item(&id).await {
                Ok(item) => {
                    if let Err(e) = self.discard_file(&item).await {
                        failures.push(e)
                    }
                }
                Err(e) => failures.push(e)
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(FileSystemError::MultipleErrors(failures))
        }
    }

    pub async fn discard_all(&self) -> Result<()> {
        println!("Trying to discard all known items sequentially!");
        match self.storage.list_files().await {
            Ok(metas) => {
                let ids = metas.iter().map(|m| m.id).collect::<Vec<u64>>();
                self.discard(ids).await
            },
            Err(e) => Err(e)
        }
    }

    pub async fn confirm(&self, destination_id: &u64, ids: Vec<u64>) -> Result<()> {
        println!("Trying to confirm items {:?} to {}", ids, destination_id);
        let mut failures = Vec::<FileSystemError>::new();
        for id in ids {
            match self.storage.get_item(&id).await {
                Ok(item) => {
                    match self.destinations.derive_using(destination_id, &item) {
                        Ok(dst_path) => {
                            if let Err(e) = self.confirm_file(dst_path.as_path(), &item).await {
                                failures.push(e)
                            }
                        }
                        Err(e) => failures.push(e)
                    }
                }
                Err(e) => failures.push(e)
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(FileSystemError::MultipleErrors(failures))
        }
    }

    async fn discard_file(&self, item: &MediaItemMetadata) -> Result<()> {
        let p = &item.path;
        if p.is_file() && p.exists() {
            println!("Discarding '{:?}'", p);
            std::fs::remove_file(p)?;
            self.storage.remove_if_known(p).await;
            self.thumbnails.remove(&item.id).await?;
            Ok(())
        } else {
            Err(FileSystemError::FileNotFound(p.clone()))
        }
    }

    async fn confirm_file(&self, destination_path: &Path, item: &MediaItemMetadata) -> Result<()> {
        let src = &item.path;
        let dst = destination_path;

        if src.is_file() && !dst.exists() {
            println!("Moving '{:?}' to '{:?}'", src, dst);

            match dst.parent() {
                Some(parent_dir) => {
                    if !parent_dir.exists() {
                        println!("Missing destination directory; Creating");
                        std::fs::create_dir_all(parent_dir)?
                    }
                }
                None => return Err(FileSystemError::NoParentDirectory(dst.to_path_buf()))
            }

            std::fs::copy(src, dst)?;
            std::fs::remove_file(src)?;
            self.storage.remove_if_known(src).await;
            self.thumbnails.remove(&item.id).await?;
            Ok(())
        } else {
            Err(FileSystemError::InvalidParameters(format!("Can't move '{:?}' to '{:?}'", src, dst)))
        }
    }
}

impl From<std::io::Error> for FileSystemError {
    fn from(e: std::io::Error) -> Self {
        FileSystemError::IOError(e.to_string())
    }
}

impl From<image::ImageError> for FileSystemError {
    fn from(e: image::ImageError) -> Self {
        FileSystemError::ImageError(format!("{:?}", e))
    }
}