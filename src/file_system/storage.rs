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

use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::file_system::model::MediaItemMetadata;
use crate::file_system::{Result, FileSystemError};

type Dt = chrono::DateTime<chrono::Utc>;

#[derive(Clone)]
pub struct MediaItemMetadataStorage(Arc<RwLock<MediaItemMetadataStorageInternal>>);

impl MediaItemMetadataStorage {
    pub fn new() -> Self {
        MediaItemMetadataStorage(Arc::new(RwLock::new(MediaItemMetadataStorageInternal::new())))
    }

    pub async fn get_item(&self, id : &u64) -> Result<MediaItemMetadata> {
        self.0.read().await.get_item(id).await
    }

    pub async fn is_path_known(&self, path : &Path) -> bool {
        self.0.read().await.is_path_known(path).await
    }

    pub async fn add_file(&self, path : &Path, name : String, mime : String, creation_date : Dt) -> Result<MediaItemMetadata> {
        self.0.write().await.add(path, name, mime, creation_date).await
    }

    pub async fn remove_file(&self, id : &u64) -> Result<()> {
        self.0.write().await.remove(id).await
    }

    pub async fn remove_path(&self, path : &Path) -> Result<()> {
        self.0.write().await.remove_path(path).await
    }

    pub async fn remove_if_known(&self, path : &Path) {
        let mut inner = self.0.write().await;
        if inner.is_path_known(path).await {
            let _r = inner.remove_path(path).await;
        }
    }

    pub async fn list_files(&self) -> Result<Vec<MediaItemMetadata>> {
        self.0.read().await.list().await
    }
}

struct MediaItemMetadataStorageInternal {
    files : HashMap<u64, MediaItemMetadata>,
    path_idx : HashMap<PathBuf, u64>,
    next_id : u64
}
impl MediaItemMetadataStorageInternal {
    pub fn new() -> Self {
        MediaItemMetadataStorageInternal {
            files: HashMap::new(),
            path_idx: HashMap::new(),
            next_id: 0
        }
    }

    pub async fn get_item(&self, id : &u64) -> Result<MediaItemMetadata> {
        match self.files.get(id) {
            Some(item) => Ok(item.clone()),
            None => Err(FileSystemError::UnknownId(*id))
        }
    }

    pub async fn is_path_known(&self, path : &Path) -> bool {
        self.path_idx.contains_key(path)
    }

    pub async fn add(&mut self, path : &Path, name : String, mime : String, creation_date : Dt) -> Result<MediaItemMetadata> {
        debug_assert!(!self.files.contains_key(&self.next_id));
        debug_assert!(!self.path_idx.contains_key(&path.to_path_buf()));

        let id = self.next_id;
        self.next_id += 1;

        let value = MediaItemMetadata{
            id, name, mime, path: path.to_path_buf(), creation_date
        };

        println!("Adding item {:?} to storage", value);

        self.files.insert(id, value.clone());
        self.path_idx.insert(path.to_path_buf(), id);

        Ok(value)
    }

    pub async fn remove(&mut self, id : &u64) -> Result<()> {
        match self.files.remove(&id) {
            Some(item) => {
                self.path_idx.remove(&item.path).expect("Removing Item without removing from path index!");
                println!("Removed item {:?} from storage", item);
                Ok(())
            },
            None => Err(FileSystemError::UnknownId(id.clone()))
        }
    }

    pub async fn remove_path(&mut self, path : &Path) -> Result<()> {
        match self.path_idx.remove(path) {
            Some(id) => {
                self.files.remove(&id).expect("Removing Path without identifying the corresponding MediaItemMetadata!");
                println!("Removed item {:?} using path {:?}", id, path);
                Ok(())
            },
            None => {
                Err(FileSystemError::UnknownPath(path.to_path_buf()))
            }
        }
    }

    pub async fn list(&self) -> Result<Vec<MediaItemMetadata>> {
        Ok(self.files.values().map(|it : &MediaItemMetadata| it.clone()).collect::<Vec<MediaItemMetadata>>())
    }
}