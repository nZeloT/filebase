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

use serde::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::reply::{with_header, with_status};

use crate::file_system::FileSystem;

const APPL_JSON: &str = "application/json";
const TEXT_PLN: &str = "text/plain";
const IMAGE_JPG: &str = "image/jpeg";

#[derive(Serialize, Deserialize, Debug)]
pub struct DiscardMediaItems {
    ids: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfirmMediaItems {
    destination: u64,
    ids: Vec<u64>,
}

pub async fn handle_list_items(fs: FileSystem) -> Result<impl warp::Reply, std::convert::Infallible> {
    match fs.list().await {
        Ok(items) => {
            Ok(reply(json(&items), APPL_JSON, StatusCode::OK))
        }
        Err(e) => {
            Ok(reply(json(&e), APPL_JSON, StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

pub async fn handle_load_item(image_id: u64, fs: FileSystem) -> Result<impl warp::Reply, std::convert::Infallible> {
    match fs.read(image_id).await {
        Ok(data) => {
            Ok(reply(data, IMAGE_JPG, StatusCode::OK))
        }
        Err(e) => Ok(reply(json(&e), APPL_JSON, StatusCode::INTERNAL_SERVER_ERROR))
    }
}

pub async fn handle_discard_items(fs: FileSystem, body: DiscardMediaItems) -> Result<impl warp::Reply, std::convert::Infallible> {
    match fs.discard(body.ids).await {
        Ok(_) => Ok(reply("".to_string().into_bytes(), TEXT_PLN, StatusCode::OK)),
        Err(e) => Ok(reply(json(&e), APPL_JSON, StatusCode::INTERNAL_SERVER_ERROR))
    }
}

pub async fn handle_confirm_items(fs: FileSystem, body: ConfirmMediaItems) -> Result<impl warp::Reply, std::convert::Infallible> {
    match fs.confirm(&body.destination, body.ids).await {
        Ok(_) => Ok(reply("".to_string().into_bytes(), TEXT_PLN, StatusCode::OK)),
        Err(e) => Ok(reply(json(&e), APPL_JSON, StatusCode::INTERNAL_SERVER_ERROR))
    }
}

pub async fn handle_list_destinations(fs: FileSystem) -> Result<impl warp::Reply, std::convert::Infallible> {
    Ok(reply(json(&fs.list_confirm_destinations().await), APPL_JSON, StatusCode::OK))
}

fn reply(response: Vec<u8>, ctype: &str, rcode: StatusCode) -> impl warp::Reply {
    with_status(with_header(with_header(response, warp::http::header::CONTENT_TYPE, ctype), warp::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"), rcode)
}

fn json<V: Serialize>(val: &V) -> Vec<u8> {
    match serde_json::to_string(val) {
        Ok(v) => v.into_bytes(),
        Err(e) => e.to_string().into_bytes()
    }
}