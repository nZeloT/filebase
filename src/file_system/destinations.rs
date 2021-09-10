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

use std::fs::File;
use std::io::BufReader;
use std::path::{PathBuf, Path};
use serde::{Deserialize, Serialize};
use serde_json;

use crate::file_system::{FileSystemError, Result};
use crate::file_system::model::MediaItemMetadata;
use chrono::Datelike;

#[derive(Serialize, Deserialize)]
pub struct FileSystemDestination {
    pub id: u64,
    pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
struct FileSystemDestinationInternal {
    #[serde(skip)]
    pub id: u64,

    pub name: String,
    base_path: PathBuf,
    dynamic_bp_suffix: String,
}

impl FileSystemDestinationInternal {
    pub fn derive_path(&self, item: &MediaItemMetadata) -> PathBuf {
        // %year%
        // %month%
        // '/mnt/data/Pictures/%year%/%month%/'
        let mut folder = if self.dynamic_bp_suffix.is_empty() {
            self.base_path.clone()
        }else{
            let mut bp = self.base_path.clone();
            let mut suffix = self.dynamic_bp_suffix.clone();

            if let Some(idx) = suffix.find("%year%") {
                suffix.replace_range(idx..idx+6, item.creation_date.year().to_string().as_str())
            }

            if let Some(idx) = suffix.find("%month%") {
                let month = format!("{:02}", item.creation_date.month());
                suffix.replace_range(idx..idx+7, month.as_str())
            }

            bp.push(Path::new(&suffix));
            bp
        };
        folder.push(Path::new(item.name.as_str()));
        folder
    }
}

pub struct FileSystemDestinations(Vec<FileSystemDestinationInternal>);

impl FileSystemDestinations {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let file = File::open(path).expect("Failed to open the given file with FileSystemDestinations");
        let reader = BufReader::new(file);
        let items : Vec<FileSystemDestinationInternal> = serde_json::from_reader(reader).expect("Failed to parse the given FileSystemDestinations!");

        let mut s = FileSystemDestinations(Vec::new());
        for mut item in items {
            item.id = s.0.len() as u64;
            println!("Adding destination {:?}", item);
            s.0.push(item);
        }

        s
    }

    pub fn derive_using(&self, id: &u64, item: &MediaItemMetadata) -> Result<PathBuf> {
        if let Some(dst) = self.0.get((*id) as usize) {
            Ok(dst.derive_path(item))
        } else {
            Err(FileSystemError::UnknownId(id.clone()))
        }
    }

    pub fn list(&self) -> Vec<FileSystemDestination> {
        self.0.iter()
            .map(|e| FileSystemDestination::from(e))
            .collect::<Vec<FileSystemDestination>>()
    }
}

impl From<&FileSystemDestinationInternal> for FileSystemDestination {
    fn from(fsi: &FileSystemDestinationInternal) -> Self {
        FileSystemDestination {
            id: fsi.id.clone(),
            name: fsi.name.clone(),
        }
    }
}