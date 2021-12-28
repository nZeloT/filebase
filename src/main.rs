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

use std::net::SocketAddr;
use std::path::Path;

mod api_handler;
pub mod file_system;

#[tokio::main]
async fn main() {
    let args : Vec<String> = std::env::args().collect();
    let src_dir = Path::new(&args[1]);
    println!("Using source directory: {:?}", src_dir);
    let dst_conf = Path::new("destination_config.json");
    let fs = file_system::FileSystem::new(src_dir, dst_conf);
    let _jh = fs.launch_watchdog(src_dir).await;

    let service = filters::endpoints(fs);

    let env_ip_str = match std::env::var("SERVER_IP") {
        Ok(given_ip) => given_ip,
        Err(_) => "192.168.2.111:5555".to_string()
    };
    let sock_address: SocketAddr = env_ip_str.parse().unwrap();

    println!("Launching filebase-server. Listening on {}", sock_address);

    warp::serve(service).run(sock_address).await
}

mod filters {
    use warp::Filter;
    use crate::api_handler;
    use crate::file_system;
    use crate::file_system::FileSystem;

    const CONTENT_LENGTH_LIMIT: u64 = 1024 * 32;

    pub fn endpoints(fs: file_system::FileSystem) -> impl warp::Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        api(fs).or(frontend())
    }

    fn frontend() -> impl warp::Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::any()
            .and(warp::fs::dir("./frontend/public"))
    }

    fn api(fs: file_system::FileSystem) -> impl warp::Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::path("api")
            .and(warp::path("v1"))
            .and(
                list_images(fs.clone())
                    .or(load_image(fs.clone()))
                    .or(confirm_images(fs.clone()))
                    .or(discard_images(fs.clone()))
                    .or(discard_all(fs.clone()))
                    .or(list_destinations(fs))
            )
    }

    fn list_images(fs: file_system::FileSystem) -> impl warp::Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::path!("items")
            .and(warp::get())
            .and(with_fs(fs))
            .and_then(api_handler::handle_list_items)
    }

    fn load_image(fs: file_system::FileSystem) -> impl warp::Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::path!("items" / "load" / u64)
            .and(warp::get())
            .and(with_fs(fs))
            .and_then(api_handler::handle_load_item)
    }

    fn confirm_images(fs: file_system::FileSystem) -> impl warp::Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::path!("items" / "confirm")
            .and(warp::post())
            .and(warp::body::content_length_limit(CONTENT_LENGTH_LIMIT))
            .and(with_fs(fs))
            .and(warp::body::json())
            .and_then(api_handler::handle_confirm_items)
    }

    fn discard_images(fs: file_system::FileSystem) -> impl warp::Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::path!("items" / "discard")
            .and(warp::post())
            .and(warp::body::content_length_limit(CONTENT_LENGTH_LIMIT))
            .and(with_fs(fs))
            .and(warp::body::json())
            .and_then(api_handler::handle_discard_items)
    }

    fn discard_all(fs: file_system::FileSystem) -> impl warp::Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::path!("items" / "discard_all")
            .and(warp::post())
            .and(with_fs(fs))
            .and_then(api_handler::handle_discard_all)
    }

    fn list_destinations(fs : FileSystem) -> impl warp::Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::path!("destinations")
            .and(warp::get())
            .and(with_fs(fs))
            .and_then(api_handler::handle_list_destinations)
    }

    fn with_fs(fs: file_system::FileSystem) -> impl Filter<Extract=(file_system::FileSystem, ), Error=std::convert::Infallible> + Clone {
        warp::any().map(move || fs.clone())
    }
}