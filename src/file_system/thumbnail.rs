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

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use image::ImageFormat;
use tokio::sync::RwLock;

use crate::file_system::{FileSystemError, Result};
use crate::file_system::model::{MediaItemMetadata};

#[derive(Clone)]
pub struct Thumbnails(Arc<RwLock<ThumbnailsInternal>>);

struct ThumbnailsInternal {
    cache_dir : PathBuf,
    cache : HashMap<u64, PathBuf>
}

impl Thumbnails {
    pub fn new(img_base_path : &Path) -> Self {
        let mut cache_dir = img_base_path.to_path_buf();
        cache_dir.push(".thumbnails");
        println!("The thumbnail cache directory is {:?}", cache_dir);
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir).expect("Failed to create thumbnail cache dir!");
            println!("The directory was created");
        }
        Thumbnails(Arc::new(RwLock::new(ThumbnailsInternal {
            cache_dir,
            cache: HashMap::new()
        })))
    }

    pub async fn load(&self, item : &MediaItemMetadata) -> Result<()> {
        self.0.write().await.load(item).await
    }

    pub async fn get(&self, id: &u64) -> Result<Vec<u8>> {
        self.0.read().await.get(id).await
    }

    pub async fn remove(&self, id: &u64) -> Result<()> {
        self.0.write().await.remove(id).await
    }
}

impl ThumbnailsInternal {

    async fn load(&mut self, item : &MediaItemMetadata) -> Result<()> {
        let target_name = format!("{}.jpg", item.id);
        let mut target_path = self.cache_dir.clone();
        target_path.push(target_name);

        println!("Generating thumbnail for file {:?} into new file {:?}", item.path, target_path);

        if let Some(_) = self.cache.insert(item.id, target_path.clone()) {
            assert!(true, "Found a already used ID!")
        }

        image::open(item.path.as_path())?
            .thumbnail(512, 512)
            .save_with_format(&target_path, ImageFormat::Jpeg)?;

        println!("Thumbnail was generated");

        Ok(())
    }

    async fn get(&self, id : &u64) -> Result<Vec<u8>> {
        println!("Requesting thumbnail {} from cache", id);
        if let Some(path) = self.cache.get(id) {
            println!("Reading in thumbnail from path {:?}", path);

            let data = std::fs::read(path)?;

            Ok(data)
        }else{
            Err(FileSystemError::UnknownId(id.clone()))
        }
    }

    async fn remove(&mut self, id : &u64) -> Result<()> {
        if let Some(path) = self.cache.get(&id) {
            std::fs::remove_file(path)?;

            self.cache.remove(&id).expect("Failed to remove from cache HashMap though id is known!");

            println!("Deleted thumbnail for id {}", id);

            Ok(())
        }else{
            Err(FileSystemError::UnknownId(id.clone()))
        }
    }

}

